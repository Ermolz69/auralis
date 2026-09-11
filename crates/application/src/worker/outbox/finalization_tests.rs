#![allow(clippy::unwrap_used)]

use super::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use domain::{media::ArtifactKind, project::Project};
use ports::{
    artifact_index::ArtifactIndex,
    repository::{OutboxRepository, ProjectRepository},
    storage::ArtifactStore,
    transaction::{CommitProjectDelete, CommitStagedArtifactWrite, StorageUnitOfWork},
};
use std::sync::Arc;

struct NoEvents;
#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected event")
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected event")
    }
}

#[tokio::test]
async fn sqlite_filesystem_worker_validates_pending_bytes_and_replay() {
    for scenario in [
        "truncated_stage",
        "wrong_final",
        "valid_final",
        "valid_stage",
        "deleted",
        "missing",
        "wrong_key",
        "unknown_size",
    ] {
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("test.sqlite"))
            .await
            .unwrap();
        let repo = SqliteProjectRepository::new(pool.clone());
        let project = repo
            .create(Project::new("Finalization".into()).unwrap())
            .await
            .unwrap();
        let base = root.path().join("artifacts");
        let store = Arc::new(LocalArtifactStore::new(base.clone()));
        let source = root.path().join("external.mp4");
        std::fs::write(&source, b"complete video").unwrap();
        let mut staged = store
            .import_external_file(project.id(), ArtifactKind::SourceVideo, &source, None)
            .await
            .unwrap();
        if scenario == "unknown_size" {
            staged.artifact.size_bytes = None;
        }
        let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
        uow.commit_staged_artifact_write(CommitStagedArtifactWrite {
            project_id: project.id().clone(),
            artifact: staged.artifact.clone(),
            staging_key: staged.staging_key.clone(),
            final_key: staged.final_key.clone(),
            temp_workspace_key: None,
        })
        .await
        .unwrap();
        let index = Arc::new(SqliteArtifactIndex::new(pool.clone()));
        assert!(index.get(&staged.artifact.id).await.unwrap().is_none());
        let outbox = SqliteOutboxRepository::new(pool.clone());
        let message = outbox.fetch_pending(10).await.unwrap().messages.remove(0);
        let final_path = base.join(&staged.final_key);
        let stage_path = base.join(&staged.staging_key);
        match scenario {
            "truncated_stage" => std::fs::write(&stage_path, b"short").unwrap(),
            "wrong_final" | "valid_final" => {
                std::fs::create_dir_all(final_path.parent().unwrap()).unwrap();
                if scenario == "wrong_final" {
                    std::fs::write(&final_path, b"wrong").unwrap();
                } else {
                    std::fs::rename(&stage_path, &final_path).unwrap();
                }
            }
            "deleted" => {
                uow.commit_project_delete(CommitProjectDelete {
                    project_id: project.id().clone(),
                })
                .await
                .unwrap();
            }
            "missing" => {
                index.delete(&staged.artifact.id).await.unwrap();
            }
            "wrong_key" => {
                sqlx::query("UPDATE artifacts SET location_value = 'different/key' WHERE id = ?")
                    .bind(staged.artifact.id.to_string())
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            _ => {}
        }
        let worker = OutboxWorker::new(
            outbox.clone(),
            store,
            index.clone(),
            uow,
            Arc::new(NoEvents),
            Arc::new(LocalTempWorkspace::new(root.path())),
            OutboxMaintenanceConfig::try_default().unwrap(),
        );
        let report = worker.process_pending_messages(20).await.unwrap();
        let invalid = matches!(
            scenario,
            "truncated_stage" | "wrong_final" | "missing" | "wrong_key"
        );
        if invalid {
            assert_eq!(report.retry_scheduled, 1, "{scenario}");
            assert!(index.get(&staged.artifact.id).await.unwrap().is_none());
            assert!(stage_path.exists());
            let status: String =
                sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
                    .bind(message.id.to_string())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(status, "failed");
        } else {
            assert_eq!(report.fetched, report.completed, "{scenario}");
            if scenario != "deleted" {
                assert!(index.get(&staged.artifact.id).await.unwrap().is_some());
                assert_eq!(std::fs::read(&final_path).unwrap(), b"complete video");
            } else {
                assert!(!final_path.exists());
            }
            worker
                .handler
                .process_payload(&message.id, &message.payload)
                .await
                .unwrap();
            assert_eq!(
                worker.process_pending_messages(20).await.unwrap().fetched,
                0
            );
        }
        assert_eq!(std::fs::read(&source).unwrap(), b"complete video");
        pool.close().await;
    }
}
