#[path = "support/translation_history_fixture.rs"]
mod history;
#[path = "support/legacy_outbox.rs"]
mod legacy_outbox;

use adapters_storage::sqlite::{SqliteTranslationReviewStore, connect_sqlite};
use domain::chrono::Utc;
use domain::project::ProjectId;
use history::HistoryFixture;
use ports::translation_links::TranslationLinkStore;
use ports::translation_reviews::{
    RecordTranslationReview, ReviewDecisionKind, TranslationReviewRecord, TranslationReviewStore,
};
use std::error::Error;
use uuid::Uuid;

#[tokio::test]
async fn review_history_is_digest_bound_and_not_inherited_by_an_edit() -> Result<(), Box<dyn Error>>
{
    let fixture = HistoryFixture::new().await?;
    let first = fixture.ready(1).await?;
    let store = SqliteTranslationReviewStore::new(fixture.pool.clone());
    let revision = fixture.link().await?.revision;
    let approved = request(
        &fixture,
        first.result_id,
        revision,
        ReviewDecisionKind::Approved,
    );
    let saved = store.record(approved).await?;
    let repeated = store
        .record(request(
            &fixture,
            first.result_id,
            revision,
            ReviewDecisionKind::Approved,
        ))
        .await?;
    assert_eq!(saved, repeated);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translation_review_events")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(count, 1);
    assert!(
        sqlx::query("UPDATE translation_review_events SET note = 'changed' WHERE event_id = ?")
            .bind(saved.event_id.to_string())
            .execute(&fixture.pool)
            .await
            .is_err()
    );
    assert!(
        store
            .selected_approval_matches(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id,
                &saved.result_sha256,
                revision
            )
            .await?
    );
    assert!(
        !store
            .selected_approval_matches(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id,
                &"f".repeat(64),
                revision
            )
            .await?
    );

    let second = fixture.ready(2).await?;
    let second_revision = fixture.link().await?.revision;
    assert!(
        !store
            .selected_approval_matches(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id,
                &saved.result_sha256,
                second_revision
            )
            .await?
    );
    assert!(
        store
            .latest(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                second.result_id
            )
            .await?
            .is_none()
    );
    assert_eq!(
        store
            .latest(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id
            )
            .await?,
        Some(saved.clone())
    );
    assert!(
        store
            .record(request(
                &fixture,
                first.result_id,
                revision,
                ReviewDecisionKind::Withdrawn
            ))
            .await
            .is_err()
    );

    let selected = fixture
        .store
        .select_historical_result(fixture.selection(first.result_id, second_revision))
        .await?;
    let current_revision = selected.revision;
    assert!(
        store
            .selected_approval_matches(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id,
                &saved.result_sha256,
                current_revision
            )
            .await?
    );
    let withdrawn = store
        .record(request(
            &fixture,
            first.result_id,
            current_revision,
            ReviewDecisionKind::Withdrawn,
        ))
        .await?;
    assert_eq!(withdrawn.decision, ReviewDecisionKind::Withdrawn);
    assert_ne!(withdrawn.event_id, saved.event_id);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translation_review_events")
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(count, 2);
    assert_eq!(
        store
            .latest(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id
            )
            .await?,
        Some(withdrawn)
    );
    assert!(
        !store
            .selected_approval_matches(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                first.result_id,
                &saved.result_sha256,
                current_revision
            )
            .await?
    );
    Ok(())
}

#[tokio::test]
async fn v10_migration_keeps_ready_result_without_implicit_approval() -> Result<(), Box<dyn Error>>
{
    let fixture = HistoryFixture::new().await?;
    let publication = fixture.ready(1).await?;
    legacy_outbox::restore_legacy_outbox_shape(&fixture.pool).await?;
    sqlx::raw_sql("DROP TABLE translation_review_events; DROP TABLE job_executions; DROP TABLE speech_host_checkpoints; DROP TABLE speech_host_attempts; DROP TABLE speech_run_intents;")
        .execute(&fixture.pool)
        .await?;
    sqlx::query("PRAGMA user_version = 10")
        .execute(&fixture.pool)
        .await?;
    // Reconnect through the same owned database after the legacy schema is staged.
    let database_path: String =
        sqlx::query_scalar("SELECT file FROM pragma_database_list WHERE name = 'main'")
            .fetch_one(&fixture.pool)
            .await?;
    fixture.pool.close().await;
    let pool = connect_sqlite(database_path).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(version, 20);
    let store = SqliteTranslationReviewStore::new(pool.clone());
    assert!(
        store
            .latest(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                publication.result_id
            )
            .await?
            .is_none()
    );
    let state: String =
        sqlx::query_scalar("SELECT state FROM translation_publications WHERE result_id = ?")
            .bind(publication.result_id.to_string())
            .fetch_one(&pool)
            .await?;
    assert_eq!(state, "ready");
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn pending_or_foreign_publication_cannot_gain_approval() -> Result<(), Box<dyn Error>> {
    let fixture = HistoryFixture::new().await?;
    let revision = fixture.link().await?.revision;
    let pending = fixture.stage(1, revision).await?;
    let store = SqliteTranslationReviewStore::new(fixture.pool.clone());
    assert!(
        store
            .record(request(
                &fixture,
                pending.result_id,
                revision,
                ReviewDecisionKind::Approved
            ))
            .await
            .is_err()
    );
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
        .bind(pending.artifact_id.to_string())
        .execute(&fixture.pool)
        .await?;
    fixture
        .store
        .finalize_ready_publication(fixture.intent.translation_id, pending.result_id, Utc::now())
        .await?;
    let current_revision = fixture.link().await?.revision;
    let mut foreign = request(
        &fixture,
        pending.result_id,
        current_revision,
        ReviewDecisionKind::Approved,
    );
    foreign.record.project_id = ProjectId::new();
    assert!(store.record(foreign).await.is_err());
    let mut invalid = request(
        &fixture,
        pending.result_id,
        current_revision,
        ReviewDecisionKind::Approved,
    );
    invalid.record.note = " ".into();
    assert!(store.record(invalid).await.is_err());
    assert!(
        store
            .latest(
                &fixture.intent.project_id,
                fixture.intent.translation_id,
                pending.result_id
            )
            .await?
            .is_none()
    );
    Ok(())
}

fn request(
    fixture: &HistoryFixture,
    result_id: Uuid,
    expected_link_revision: u64,
    decision: ReviewDecisionKind,
) -> RecordTranslationReview {
    RecordTranslationReview {
        record: TranslationReviewRecord {
            event_id: Uuid::new_v4(),
            project_id: fixture.intent.project_id.clone(),
            translation_id: fixture.intent.translation_id,
            result_id,
            result_sha256: "b".repeat(64),
            decision,
            reviewer_label: "Local reviewer".into(),
            note: "Checked source and translation".into(),
            created_at: Utc::now(),
        },
        expected_link_revision,
    }
}
