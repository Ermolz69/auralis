#[path = "support/translation_history_fixture.rs"]
mod fixture;
#[path = "support/legacy_outbox.rs"]
mod legacy_outbox;

use adapters_storage::sqlite::{SqliteTranslationEditIntentStore, connect_sqlite};
use domain::chrono::Utc;
use domain::project::ProjectId;
use fixture::HistoryFixture;
use ports::error::PortError;
use ports::translation_edit_intents::{TranslationEditIntent, TranslationEditIntentStore};
use ports::translation_links::{StartTranslationRun, TranslationLinkStore};
use sqlx::SqlitePool;
use std::error::Error;
use uuid::Uuid;

async fn freeze(f: &HistoryFixture) -> Result<(), Box<dyn Error>> {
    sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES (?, ?, '{}', ?, 1, 0, 0, 1, ?, ?)")
        .bind(f.intent.run_id.to_string()).bind(f.intent.translation_id.to_string())
        .bind("b".repeat(64)).bind("c".repeat(64)).bind(Utc::now().to_rfc3339())
        .execute(&f.pool).await?;
    Ok(())
}

async fn request(f: &HistoryFixture) -> Result<TranslationEditIntent, Box<dyn Error>> {
    freeze(f).await?;
    let first = f.ready(1).await?;
    let second = f.ready(2).await?;
    Ok(TranslationEditIntent {
        project_id: f.intent.project_id.clone(),
        translation_id: f.intent.translation_id,
        result_id: Uuid::new_v4(),
        base_result_id: first.result_id,
        run_id: f.intent.run_id,
        expected_head_result_id: second.result_id,
        expected_link_revision: f.link().await?.revision,
        request_sha256: "d".repeat(64),
        created_at: Utc::now(),
    })
}

#[tokio::test]
async fn edit_intent_survives_selection_another_active_run_and_reopen_without_text()
-> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let intent = request(&f).await?;
    let store = SqliteTranslationEditIntentStore::new(f.pool.clone());
    store.record(intent.clone()).await?;
    f.store
        .select_historical_result(f.selection(intent.base_result_id, intent.expected_link_revision))
        .await?;
    let link = f.link().await?;
    let started = f
        .store
        .start_run(StartTranslationRun {
            translation_id: intent.translation_id,
            run_id: Uuid::new_v4(),
            expected_active_run_id: link.active_run_id,
            expected_link_revision: link.revision,
            updated_at: Utc::now(),
        })
        .await?;
    let mut retry = intent.clone();
    retry.created_at = Utc::now();
    store.record(retry.clone()).await?;
    retry.request_sha256 = "e".repeat(64);
    assert!(matches!(
        store.record(retry).await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(f.link().await?, started);
    let database: String =
        sqlx::query_scalar("SELECT file FROM pragma_database_list WHERE name = 'main'")
            .fetch_one(&f.pool)
            .await?;
    f.pool.close().await;
    let reopened = connect_sqlite(&database).await?;
    let reopened_store = SqliteTranslationEditIntentStore::new(reopened.clone());
    assert_eq!(
        reopened_store.get(intent.result_id).await?,
        Some(intent.clone())
    );
    assert_eq!(reopened_store.list_unpublished(None, 100).await?, [intent]);
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('translation_edit_intents')")
            .fetch_all(&reopened)
            .await?;
    assert_eq!(columns.len(), 9);
    assert!(
        !columns
            .iter()
            .any(|column| column.contains("text") || column.contains("lines"))
    );
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn invalid_or_foreign_edit_intents_do_not_create_rows() -> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let intent = request(&f).await?;
    let store = SqliteTranslationEditIntentStore::new(f.pool.clone());
    let mut foreign = intent.clone();
    foreign.project_id = ProjectId::new();
    let mut stale = intent.clone();
    stale.expected_link_revision -= 1;
    let mut missing = intent.clone();
    missing.base_result_id = Uuid::new_v4();
    let mut wrong_run = intent.clone();
    wrong_run.run_id = Uuid::new_v4();
    let mut malformed = intent.clone();
    malformed.request_sha256 = "D".repeat(64);
    let mut nil = intent.clone();
    nil.expected_head_result_id = Uuid::nil();
    let mut existing = intent;
    existing.result_id = existing.expected_head_result_id;
    for invalid in [foreign, stale, missing, wrong_run, malformed, nil, existing] {
        assert!(store.record(invalid).await.is_err());
    }
    assert!(store.list_unpublished(None, 100).await?.is_empty());
    assert!(store.list_unpublished(None, 0).await.is_err());
    assert!(store.list_unpublished(None, 101).await.is_err());
    f.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn concurrent_changed_requests_for_one_result_id_have_one_durable_winner()
-> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let first = request(&f).await?;
    let mut second = first.clone();
    second.request_sha256 = "e".repeat(64);
    let store = SqliteTranslationEditIntentStore::new(f.pool.clone());
    let (left, right) = tokio::join!(store.record(first.clone()), store.record(second.clone()));
    let winner = match (left, right) {
        (Ok(()), Err(PortError::Conflict { .. })) => first,
        (Err(PortError::Conflict { .. }), Ok(())) => second,
        _ => return Err("concurrent requests did not have one winner".into()),
    };
    assert_eq!(
        store.list_unpublished(None, 100).await?,
        std::slice::from_ref(&winner)
    );
    store.record(winner).await?;
    f.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn ready_unattached_edits_leave_recovery_and_translation_deletion_cascades_intents()
-> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let intent = request(&f).await?;
    let store = SqliteTranslationEditIntentStore::new(f.pool.clone());
    store.record(intent.clone()).await?;
    let selected = f.link().await?;
    let publication = f.stage(3, selected.revision).await?;
    sqlx::query(
        "UPDATE translation_publications SET result_id = ?, state = 'ready' WHERE result_id = ?",
    )
    .bind(intent.result_id.to_string())
    .bind(publication.result_id.to_string())
    .execute(&f.pool)
    .await?;
    assert_eq!(
        store.list_unpublished(None, 100).await?,
        std::slice::from_ref(&intent)
    );
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
        .bind(publication.artifact_id.to_string())
        .execute(&f.pool)
        .await?;
    assert!(store.list_unpublished(None, 100).await?.is_empty());
    assert_eq!(f.link().await?, selected);
    assert_eq!(store.get(intent.result_id).await?, Some(intent.clone()));
    sqlx::query("DELETE FROM project_translations WHERE translation_id = ?")
        .bind(intent.translation_id.to_string())
        .execute(&f.pool)
        .await?;
    assert_eq!(store.get(intent.result_id).await?, None);
    let violations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
        .fetch_one(&f.pool)
        .await?;
    assert_eq!(violations, 0);
    f.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn v9_upgrade_is_idempotent_preserves_history_and_rejects_corrupt_intent_metadata()
-> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let intent = request(&f).await?;
    let before = f.link().await?;
    let database: String =
        sqlx::query_scalar("SELECT file FROM pragma_database_list WHERE name = 'main'")
            .fetch_one(&f.pool)
            .await?;
    legacy_outbox::restore_legacy_outbox_shape(&f.pool).await?;
    sqlx::raw_sql(
        "DROP TABLE translation_review_events; DROP TABLE job_executions; DROP TABLE speech_host_checkpoints; DROP TABLE speech_host_attempts; DROP TABLE speech_run_intents; DROP TABLE translation_edit_intents; PRAGMA user_version = 9;",
    )
        .execute(&f.pool)
        .await?;
    f.pool.close().await;
    let pool = connect_sqlite(&database).await?;
    assert_eq!(version(&pool).await?, 18);
    let links = adapters_storage::sqlite::SqliteTranslationLinkStore::new(pool.clone());
    assert_eq!(links.get(intent.translation_id).await?, Some(before));
    let store = SqliteTranslationEditIntentStore::new(pool.clone());
    assert!(store.list_unpublished(None, 100).await?.is_empty());
    store.record(intent.clone()).await?;
    sqlx::query("UPDATE translation_edit_intents SET request_sha256 = ? WHERE result_id = ?")
        .bind("D".repeat(64))
        .bind(intent.result_id.to_string())
        .execute(&pool)
        .await?;
    assert!(matches!(
        store.get(intent.result_id).await,
        Err(PortError::InvalidStoredData { .. })
    ));
    pool.close().await;
    let reopened = connect_sqlite(&database).await?;
    assert_eq!(version(&reopened).await?, 18);
    sqlx::query("PRAGMA user_version = 19")
        .execute(&reopened)
        .await?;
    reopened.close().await;
    assert!(connect_sqlite(&database).await.is_err());
    Ok(())
}

async fn version(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn pending_intents_can_be_paged_without_starving_later_work() -> Result<(), Box<dyn Error>> {
    let f = HistoryFixture::new().await?;
    let first = request(&f).await?;
    let mut second = first.clone();
    second.result_id = Uuid::new_v4();
    second.created_at += chrono::Duration::seconds(1);
    let store = SqliteTranslationEditIntentStore::new(f.pool.clone());
    store.record(first.clone()).await?;
    store.record(second.clone()).await?;
    assert_eq!(
        store.list_unpublished(None, 1).await?,
        std::slice::from_ref(&first)
    );
    assert_eq!(
        store.list_unpublished(Some(first.result_id), 1).await?,
        std::slice::from_ref(&second)
    );
    assert!(
        store
            .list_unpublished(Some(second.result_id), 1)
            .await?
            .is_empty()
    );
    assert!(
        store
            .list_unpublished(Some(Uuid::new_v4()), 1)
            .await
            .is_err()
    );
    f.pool.close().await;
    Ok(())
}
