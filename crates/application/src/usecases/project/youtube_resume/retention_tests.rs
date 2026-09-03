#![allow(clippy::unwrap_used)]

use super::super::youtube_storage_support::UnusedEventPublisher;
use super::{fixture::Fixture, tests::kill_at};
use crate::worker::outbox::{
    OutboxWorker,
    maintenance::{MaintenanceCoordinator, MaintenanceStepOutcome, OutboxMaintenanceConfig},
};
use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteOutboxRepository, SqliteStorageUnitOfWork,
};
use domain::project::Project;
use ports::{
    repository::ProjectRepository,
    storage::ArtifactStore,
    transaction::{CommitProjectDelete, StorageUnitOfWork},
    workspace::TempWorkspacePort,
    youtube_import::YoutubeImportJournal,
};
use std::{sync::Arc, time::Duration};

async fn maintenance(fixture: &Fixture) {
    let mut config = OutboxMaintenanceConfig::try_default().unwrap();
    config.staging_max_age = Duration::ZERO;
    config.workspace_max_age = Duration::ZERO;
    let coordinator = MaintenanceCoordinator::new(
        SqliteOutboxRepository::new(fixture.pool.clone()),
        fixture.store.clone(),
        fixture.workspace.clone(),
        config,
    )
    .with_imports(Some(fixture.journal.clone()));
    let (_tx, rx) = tokio::sync::watch::channel(false);
    let report = coordinator.run_maintenance(rx).await;
    assert_eq!(report.staging_cleanup, MaintenanceStepOutcome::Succeeded);
    assert_eq!(report.workspace_cleanup, MaintenanceStepOutcome::Succeeded);
}

async fn drain(fixture: &Fixture) {
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(fixture.pool.clone()),
        fixture.store.clone(),
        Arc::new(SqliteArtifactIndex::new(fixture.pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(fixture.pool.clone())),
        Arc::new(UnusedEventPublisher),
        fixture.workspace.clone(),
        OutboxMaintenanceConfig::try_default().unwrap(),
    );
    let report = worker.process_pending_messages(50).await.unwrap();
    assert_eq!(report.fetched, report.completed);
}

#[tokio::test]
async fn youtube_resume_maintenance_protects_checkpoints_and_committed_outbox_files() {
    let root = tempfile::tempdir().unwrap();
    kill_at(root.path(), "stage-after-checkpoint");
    let fixture = Fixture::open(root.path()).await;
    let session = fixture.journal.list().await.unwrap().remove(0);
    let stage = session.write.as_ref().unwrap();
    maintenance(&fixture).await;
    assert!(
        fixture
            .store
            .verify_staging(&stage.staging_key, stage.artifact.size_bytes.unwrap())
            .await
            .unwrap()
    );
    assert!(
        fixture
            .workspace
            .resolve_key(&session.workspace_key)
            .await
            .unwrap()
            .exists()
    );
    fixture.usecase().resume(&session.project.id).await.unwrap();
    assert!(fixture.journal.list().await.unwrap().is_empty());
    maintenance(&fixture).await;
    assert!(
        fixture
            .store
            .verify_staging(&stage.staging_key, stage.artifact.size_bytes.unwrap())
            .await
            .unwrap()
    );
    drain(&fixture).await;
    fixture.pool.close().await;
}

#[tokio::test]
async fn youtube_resume_missing_stage_reuses_the_completed_download() {
    let root = tempfile::tempdir().unwrap();
    kill_at(root.path(), "stage-after-checkpoint");
    let fixture = Fixture::open(root.path()).await;
    let session = fixture.journal.list().await.unwrap().remove(0);
    fixture
        .store
        .delete_storage_key(&session.write.unwrap().staging_key)
        .await
        .unwrap();
    fixture.usecase().resume(&session.project.id).await.unwrap();
    drain(&fixture).await;
    assert_eq!(
        std::fs::read(root.path().join("network-completions")).unwrap(),
        b"1"
    );
    fixture.pool.close().await;
}

#[tokio::test]
async fn youtube_resume_discard_and_project_delete_remove_the_journal_and_enqueue_cleanup() {
    for delete in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let fixture = Fixture::open(root.path()).await;
        let original = fixture
            .repo
            .create(Project::new("Keep".into()).unwrap())
            .await
            .unwrap();
        fixture.pool.close().await;
        kill_at(root.path(), "stage-after-checkpoint");
        let fixture = Fixture::open(root.path()).await;
        let session = fixture.journal.list().await.unwrap().remove(0);
        let staging_key = &session.write.as_ref().unwrap().staging_key;
        let allocation = fixture
            .workspace
            .resolve_key(&session.workspace_key)
            .await
            .unwrap();
        if delete {
            SqliteStorageUnitOfWork::new(fixture.pool.clone())
                .commit_project_delete(CommitProjectDelete {
                    project_id: original.id().clone(),
                })
                .await
                .unwrap();
        } else {
            fixture.usecase().discard(original.id()).await.unwrap();
        }
        assert!(fixture.journal.list().await.unwrap().is_empty());
        assert!(allocation.exists(), "cleanup must not precede the outbox");
        assert!(fixture.usecase().resume(original.id()).await.is_err());
        drain(&fixture).await;
        assert!(!allocation.exists());
        assert!(!fixture.store.verify_staging(staging_key, 13).await.unwrap());
        assert_eq!(
            fixture.repo.get(original.id()).await.unwrap(),
            if delete { None } else { Some(original) }
        );
        fixture.pool.close().await;
    }
}

#[tokio::test]
async fn youtube_resume_checkpoint_and_discard_use_compare_and_swap() {
    let root = tempfile::tempdir().unwrap();
    kill_at(root.path(), "stage-after-checkpoint");
    let fixture = Fixture::open(root.path()).await;
    let mut session = fixture.journal.list().await.unwrap().remove(0);
    let mut stale = session.clone();
    fixture.journal.checkpoint(&mut session).await.unwrap();
    assert!(fixture.journal.checkpoint(&mut stale).await.is_err());
    assert!(
        fixture
            .journal
            .discard(&stale.project.id, stale.revision)
            .await
            .is_err()
    );
    assert_eq!(
        fixture.journal.list().await.unwrap()[0].revision,
        session.revision
    );
    fixture
        .usecase()
        .discard(&session.project.id)
        .await
        .unwrap();
    drain(&fixture).await;
    fixture.pool.close().await;
}
