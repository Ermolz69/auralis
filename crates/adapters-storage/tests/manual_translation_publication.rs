#[path = "support/manual_publication_fixture.rs"]
mod fixture;
#[path = "support/translation_history_fixture.rs"]
mod history;

use adapters_storage::sqlite::{SqliteTranslationLinkStore, connect_sqlite};
use domain::chrono::Utc;
use fixture::ManualPublicationFixture;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use std::error::Error;

#[tokio::test]
async fn manual_output_is_not_attached_before_ready_and_finishes_once() -> Result<(), Box<dyn Error>>
{
    let f = ManualPublicationFixture::new().await?;
    let before = f.history.link().await?;
    f.stage().await?;
    f.stage().await?;
    assert!(f.finish().await.is_err());
    assert!(
        f.history
            .store
            .list_ready_publications(100)
            .await?
            .is_empty()
    );
    assert_eq!(f.history.link().await?, before);
    f.mark_ready().await?;
    assert_eq!(
        f.history.store.list_ready_publications(100).await?[0].result_id,
        f.edit.result_id
    );
    let attached = f.finish().await?;
    assert_eq!(attached.selected_result_id, Some(f.edit.result_id));
    assert_eq!(attached.active_run_id, before.active_run_id);
    assert_eq!(attached.revision, before.revision + 1);
    assert_eq!(f.finish().await?, attached);
    assert!(
        f.history
            .store
            .list_ready_publications(100)
            .await?
            .is_empty()
    );
    assert_eq!(
        f.history
            .store
            .get_publication(f.edit.translation_id, f.edit.result_id)
            .await?
            .ok_or("publication missing")?
            .state,
        TranslationPublicationState::Ready
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM artifacts WHERE id = ?), (SELECT COUNT(*) FROM outbox_messages WHERE deduplication_key = ?)")
        .bind(f.publication.artifact_id.to_string()).bind(format!("translation-publication:{}:{}", f.edit.translation_id, f.edit.result_id)).fetch_one(&f.history.pool).await?;
    assert_eq!(counts, (1, 1));
    f.history.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn changed_selection_and_new_active_run_keep_manual_result_ready_but_unattached_after_reopen()
-> Result<(), Box<dyn Error>> {
    let mut f = ManualPublicationFixture::new().await?;
    f.choose_base().await?;
    let before = f.begin_other_run().await?;
    f.stage().await?;
    f.mark_ready().await?;
    assert_eq!(f.finish().await?, before);
    let ready = f
        .history
        .store
        .list_ready_history(&f.edit.project_id, f.edit.translation_id, None, 100)
        .await?;
    assert!(
        ready
            .entries
            .iter()
            .any(|p| p.result_id == f.edit.result_id)
    );
    let database: String =
        sqlx::query_scalar("SELECT file FROM pragma_database_list WHERE name = 'main'")
            .fetch_one(&f.history.pool)
            .await?;
    f.history.pool.close().await;
    let pool = connect_sqlite(database).await?;
    let store = SqliteTranslationLinkStore::new(pool.clone());
    assert_eq!(
        store
            .finalize_ready_publication(f.edit.translation_id, f.edit.result_id, Utc::now())
            .await?,
        before
    );
    let chosen = store
        .select_historical_result(f.history.selection(f.edit.result_id, before.revision))
        .await?;
    assert_eq!(chosen.selected_result_id, Some(f.edit.result_id));
    assert_eq!(chosen.active_run_id, before.active_run_id);
    assert_eq!(
        store
            .finalize_ready_publication(f.edit.translation_id, f.edit.result_id, Utc::now())
            .await?,
        chosen
    );
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn pending_automatic_publication_wins_without_manual_precedence_blocking_it()
-> Result<(), Box<dyn Error>> {
    let f = ManualPublicationFixture::new().await?;
    let before = f.history.link().await?;
    let automatic = f.history.stage(2, before.revision).await?;
    f.stage().await?;
    f.mark_ready().await?;
    assert_eq!(f.finish().await?, before);
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
        .bind(automatic.artifact_id.to_string())
        .execute(&f.history.pool)
        .await?;
    let candidates = f.history.store.list_ready_publications(100).await?;
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].result_id, automatic.result_id);
    let attached = f
        .history
        .store
        .finalize_ready_publication(f.edit.translation_id, automatic.result_id, Utc::now())
        .await?;
    assert_eq!(attached.selected_result_id, Some(automatic.result_id));
    assert_eq!(f.finish().await?, attached);
    assert_eq!(
        f.history
            .store
            .list_ready_history(&f.edit.project_id, f.edit.translation_id, None, 100)
            .await?
            .entries
            .len(),
        4
    );
    f.history.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn manual_attachment_preserves_an_active_frozen_run_and_allows_current_revision_restaging()
-> Result<(), Box<dyn Error>> {
    let mut f = ManualPublicationFixture::new().await?;
    let active = f.begin_other_run().await?;
    let mut edit = f.edit.clone();
    edit.result_id = uuid::Uuid::new_v4();
    edit.expected_link_revision = active.revision;
    adapters_storage::sqlite::SqliteTranslationEditIntentStore::new(f.history.pool.clone())
        .record(edit.clone())
        .await?;
    f.edit = edit;
    f.publication.result_id = f.edit.result_id;
    f.publication.expected_link_revision = f.edit.expected_link_revision;
    f.stage().await?;
    f.mark_ready().await?;
    let attached = f.finish().await?;
    assert_eq!(attached.active_run_id, active.active_run_id);
    assert_eq!(attached.selected_result_id, Some(f.edit.result_id));
    assert!(f.history.stage(1, active.revision).await.is_err());
    let automatic = f.history.stage(1, attached.revision).await?;
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
        .bind(automatic.artifact_id.to_string())
        .execute(&f.history.pool)
        .await?;
    let final_link = f
        .history
        .store
        .finalize_ready_publication(f.edit.translation_id, automatic.result_id, Utc::now())
        .await?;
    assert_eq!(final_link.selected_result_id, Some(automatic.result_id));
    assert_eq!(final_link.active_run_id, None);
    f.history.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn invalid_manual_metadata_and_unready_base_roll_back_artifact_and_outbox()
-> Result<(), Box<dyn Error>> {
    let f = ManualPublicationFixture::new().await?;
    let mut wrong = f.publication.clone();
    wrong.run_id = uuid::Uuid::new_v4();
    assert!(
        f.history
            .store
            .commit_staged_publication(wrong, f.staged())
            .await
            .is_err()
    );
    let mut wrong = f.publication.clone();
    wrong.expected_link_revision += 1;
    assert!(
        f.history
            .store
            .commit_staged_publication(wrong, f.staged())
            .await
            .is_err()
    );
    let mut wrong = f.publication.clone();
    wrong.needs_review = false;
    assert!(
        f.history
            .store
            .commit_staged_publication(wrong, f.staged())
            .await
            .is_err()
    );
    sqlx::query("UPDATE artifacts SET state = 'pending_finalize' WHERE id = (SELECT artifact_id FROM translation_publications WHERE result_id = ?)")
        .bind(f.edit.base_result_id.to_string()).execute(&f.history.pool).await?;
    assert!(f.stage().await.is_err());
    let artifacts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE id = ?")
        .bind(f.publication.artifact_id.to_string())
        .fetch_one(&f.history.pool)
        .await?;
    assert_eq!(artifacts, 0);
    let outbox: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages WHERE deduplication_key = ?")
            .bind(format!(
                "translation-publication:{}:{}",
                f.edit.translation_id, f.edit.result_id
            ))
            .fetch_one(&f.history.pool)
            .await?;
    assert_eq!(outbox, 0);
    assert_eq!(
        f.history
            .store
            .get_publication(f.edit.translation_id, f.edit.result_id)
            .await?,
        None
    );
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = (SELECT artifact_id FROM translation_publications WHERE result_id = ?)")
        .bind(f.edit.base_result_id.to_string()).execute(&f.history.pool).await?;
    f.stage().await?;
    f.mark_ready().await?;
    let before = f.history.link().await?;
    sqlx::query("UPDATE translation_publications SET result_revision = 1 WHERE result_id = ?")
        .bind(f.edit.result_id.to_string())
        .execute(&f.history.pool)
        .await?;
    assert!(f.finish().await.is_err());
    assert_eq!(f.history.link().await?, before);
    assert_eq!(
        f.history
            .store
            .get_publication(f.edit.translation_id, f.edit.result_id)
            .await?
            .ok_or("missing pending publication")?
            .state,
        TranslationPublicationState::Pending
    );
    sqlx::query("UPDATE translation_publications SET result_revision = 3 WHERE result_id = ?")
        .bind(f.edit.result_id.to_string())
        .execute(&f.history.pool)
        .await?;
    assert_eq!(f.finish().await?.selected_result_id, Some(f.edit.result_id));
    f.history.pool.close().await;
    Ok(())
}

use ports::translation_edit_intents::TranslationEditIntentStore;

#[tokio::test]
async fn concurrent_manual_finalization_and_automatic_staging_are_serialized()
-> Result<(), Box<dyn Error>> {
    let mut f = ManualPublicationFixture::new().await?;
    let active = f.begin_other_run().await?;
    let mut edit = f.edit.clone();
    edit.result_id = uuid::Uuid::new_v4();
    edit.expected_link_revision = active.revision;
    adapters_storage::sqlite::SqliteTranslationEditIntentStore::new(f.history.pool.clone())
        .record(edit.clone())
        .await?;
    f.edit = edit;
    f.publication.result_id = f.edit.result_id;
    f.publication.expected_link_revision = f.edit.expected_link_revision;
    f.stage().await?;
    f.mark_ready().await?;
    let (manual, automatic) = tokio::join!(f.finish(), f.history.stage(1, active.revision));
    let manual = manual?;
    let publication = match automatic {
        Ok(publication) => {
            assert_eq!(manual, active);
            publication
        }
        Err(_) => {
            assert_eq!(manual.selected_result_id, Some(f.edit.result_id));
            assert_eq!(manual.active_run_id, active.active_run_id);
            f.history.stage(1, manual.revision).await?
        }
    };
    sqlx::query("UPDATE artifacts SET state = 'ready' WHERE id = ?")
        .bind(publication.artifact_id.to_string())
        .execute(&f.history.pool)
        .await?;
    let completed = f
        .history
        .store
        .finalize_ready_publication(f.edit.translation_id, publication.result_id, Utc::now())
        .await?;
    assert_eq!(completed.selected_result_id, Some(publication.result_id));
    assert_eq!(completed.active_run_id, None);
    assert_eq!(f.finish().await?, completed);
    f.history.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn selected_newer_manual_revision_blocks_older_automatic_admission_on_same_run()
-> Result<(), Box<dyn Error>> {
    let f = ManualPublicationFixture::new().await?;
    f.stage().await?;
    f.mark_ready().await?;
    let attached = f.finish().await?;
    assert!(f.history.stage(2, attached.revision).await.is_err());
    assert_eq!(f.history.link().await?, attached);
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM translation_publications WHERE state = 'pending'")
            .fetch_one(&f.history.pool)
            .await?;
    assert_eq!(pending, 0);
    f.history.pool.close().await;
    Ok(())
}

#[tokio::test]
async fn concurrent_manual_staging_and_history_choice_preserve_the_transaction_winner()
-> Result<(), Box<dyn Error>> {
    let f = ManualPublicationFixture::new().await?;
    let before = f.history.link().await?;
    let (staged, chosen) = tokio::join!(f.stage(), f.choose_base());
    staged?;
    f.mark_ready().await?;
    let finalized = f.finish().await?;
    match chosen {
        Ok(chosen) => {
            assert_eq!(chosen.selected_result_id, Some(f.edit.base_result_id));
            assert_eq!(finalized, chosen);
        }
        Err(error) => {
            assert!(matches!(
                error.downcast_ref::<ports::error::PortError>(),
                Some(ports::error::PortError::Busy { .. })
            ));
            assert_eq!(finalized.selected_result_id, Some(f.edit.result_id));
            assert_eq!(finalized.revision, before.revision + 1);
        }
    }
    assert_eq!(
        f.history
            .store
            .get_publication(f.edit.translation_id, f.edit.result_id)
            .await?
            .ok_or("missing manual publication")?
            .state,
        TranslationPublicationState::Ready
    );
    f.history.pool.close().await;
    Ok(())
}
