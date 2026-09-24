#![allow(clippy::unwrap_used)]

use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::{
    media::{ArtifactId, ArtifactKind},
    project::Project,
};
use ports::{
    repository::ProjectRepository,
    storage::ArtifactStore,
    translation_links::{PublicationIntent, TranslationIntent, TranslationLinkStore},
};
use std::sync::Arc;
use uuid::Uuid;

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected project event")
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected transcript event")
    }
}

#[tokio::test]
async fn selects_ready_translation_after_finalization_and_after_restart_gap() {
    for recovered_gap in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("test.sqlite"))
            .await
            .unwrap();
        let project = SqliteProjectRepository::new(pool.clone())
            .create(Project::new("Translation".into()).unwrap())
            .await
            .unwrap();
        let source_id = ArtifactId::new();
        sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
            .bind(source_id.to_string())
            .bind(project.id().to_string())
            .execute(&pool).await.unwrap();

        let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
        let translation_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let source_sha256 = "a".repeat(64);
        links
            .begin(TranslationIntent {
                translation_id,
                project_id: project.id().clone(),
                source_artifact_id: source_id,
                source_sha256: source_sha256.clone(),
                source_format: "srt".into(),
                source_language: "zh".into(),
                target_language: "ru".into(),
                run_id,
                created_at: domain::chrono::Utc::now(),
            })
            .await
            .unwrap();

        let store = Arc::new(LocalArtifactStore::new(root.path().join("artifacts")));
        let translated = root.path().join("translated.srt");
        std::fs::write(&translated, b"1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        let staged = store
            .import_external_file(
                project.id(),
                ArtifactKind::TranslatedTranscript,
                &translated,
                None,
            )
            .await
            .unwrap();
        let artifact_id = staged.artifact.id.clone();
        let staging_key = staged.staging_key.clone();
        let final_key = staged.final_key.clone();
        let size_bytes = staged.size_bytes;
        let result_id = Uuid::new_v4();
        links
            .commit_staged_publication(
                PublicationIntent {
                    translation_id,
                    result_id,
                    run_id,
                    artifact_id: artifact_id.clone(),
                    source_sha256,
                    expected_link_revision: 1,
                    needs_review: true,
                    created_at: domain::chrono::Utc::now(),
                },
                staged,
            )
            .await
            .unwrap();
        assert!(links.list_ready_publications(10).await.unwrap().is_empty());

        if recovered_gap {
            store
                .finalize_staged_artifact(&staging_key, &final_key, Some(size_bytes))
                .await
                .unwrap();
            sqlx::query("UPDATE artifacts SET state = 'ready', ready_at = 'now' WHERE id = ?")
                .bind(artifact_id.to_string())
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("UPDATE outbox_messages SET status = 'done' WHERE deduplication_key = ?")
                .bind(format!(
                    "translation-publication:{translation_id}:{result_id}"
                ))
                .execute(&pool)
                .await
                .unwrap();
            assert_eq!(links.list_ready_publications(10).await.unwrap().len(), 1);
        }

        let worker = OutboxWorker::new(
            SqliteOutboxRepository::new(pool.clone()),
            store,
            Arc::new(SqliteArtifactIndex::new(pool.clone())),
            Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
            Arc::new(NoEvents),
            Arc::new(LocalTempWorkspace::new(root.path())),
            OutboxMaintenanceConfig::try_default().unwrap(),
        )
        .with_translations(links.clone());
        let report = worker.process_pending_messages(10).await.unwrap();
        assert_eq!(report.fetched, usize::from(!recovered_gap));
        let link = links.get(translation_id).await.unwrap().unwrap();
        assert_eq!(link.selected_result_id, Some(result_id));
        assert_eq!(link.output_artifact_id, Some(artifact_id));
        assert!(link.active_run_id.is_none());
        assert!(links.list_ready_publications(10).await.unwrap().is_empty());
        assert_eq!(
            worker.process_pending_messages(10).await.unwrap().fetched,
            0
        );
        pool.close().await;
    }
}
