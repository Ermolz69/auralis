#![allow(clippy::unwrap_used)]
use super::command_tests::Capture;
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::{media::ArtifactKind, project::Project};
use ports::{
    repository::{OutboxRepository, ProjectRepository},
    storage::ArtifactStore,
    transaction::{CommitStagedArtifactWrite, StorageUnitOfWork},
};
use std::sync::Arc;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::layer::SubscriberExt;

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
async fn failed_finalize_records_the_attempt_without_source_details() {
    let root = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(root.path().join("test.sqlite"))
        .await
        .unwrap();
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("SYNTHETIC_SECRET_TITLE".into()).unwrap())
        .await
        .unwrap();
    let source = root.path().join("SYNTHETIC_SECRET_FILE.mp4");
    std::fs::write(&source, b"complete video").unwrap();
    let store = Arc::new(LocalArtifactStore::new(root.path().join("artifacts")));
    let stage = store
        .import_external_file(project.id(), ArtifactKind::SourceVideo, &source, None)
        .await
        .unwrap();
    let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
    uow.commit_staged_artifact_write(CommitStagedArtifactWrite {
        project_id: project.id().clone(),
        artifact: stage.artifact,
        staging_key: stage.staging_key.clone(),
        final_key: stage.final_key,
        temp_workspace_key: None,
    })
    .await
    .unwrap();
    std::fs::write(
        root.path().join("artifacts").join(stage.staging_key),
        b"short",
    )
    .unwrap();
    let outbox = SqliteOutboxRepository::new(pool.clone());
    let message_id = outbox.fetch_pending(10).await.unwrap().messages[0]
        .id
        .to_string();
    let worker = OutboxWorker::new(
        outbox,
        store,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        uow,
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        OutboxMaintenanceConfig::try_default().unwrap(),
    );
    let capture = Capture(Arc::default());
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(capture.clone()),
        );
    assert_eq!(
        worker
            .process_pending_messages(10)
            .with_subscriber(subscriber)
            .await
            .unwrap()
            .retry_scheduled,
        1
    );
    let output = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    let events: Vec<serde_json::Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let attempts: Vec<_> = events
        .iter()
        .filter(|event| event["fields"]["event_name"] == "outbox_attempt_completed")
        .collect();
    assert_eq!(attempts.len(), 1);
    let fields = &attempts[0]["fields"];
    assert_eq!(fields["operation_id"], message_id);
    assert_eq!(fields["attempt"], 1);
    assert_eq!(fields["outcome"], "retry_scheduled");
    assert_eq!(fields["error_code"], "Conflict");
    assert!(!output.contains("SYNTHETIC_SECRET"));
    pool.close().await;
}
