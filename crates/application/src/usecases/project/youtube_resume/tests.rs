#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::{
    create_from_youtube::CreateProjectFromYoutubeRequest,
    youtube_storage_support::UnusedEventPublisher,
};
use super::fixture::{Fixture, URL, checkpoint};
use crate::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteOutboxRepository, SqliteStorageUnitOfWork,
};
use domain::{
    media::ArtifactState,
    outbox::OutboxPayload,
    project::{Project, ProjectStatus},
};
use ports::{
    artifact_index::ArtifactIndex,
    repository::{OutboxRepository, ProjectRepository},
    storage::ArtifactStore,
    workspace::TempWorkspacePort,
    youtube_import::YoutubeImportJournal,
};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn kill_at(root: &Path, phase: &str) {
    let mut child = KillOnDrop(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "usecases::project::youtube_resume::tests::youtube_resume_crash_child",
                "--nocapture",
            ])
            .env("AURALIS_CRASH_ROOT", root)
            .env("AURALIS_CRASH_PHASE", phase)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let started = Instant::now();
    while !root.join("checkpoint").exists() {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "child exited before {phase}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "checkpoint {phase} timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    assert!(!child.0.wait().unwrap().success());
}

#[tokio::test]
async fn youtube_resume_crash_child() {
    let Ok(root) = std::env::var("AURALIS_CRASH_ROOT") else {
        return;
    };
    let fixture = Fixture::open(Path::new(&root)).await;
    let project_id = fixture
        .repo
        .list()
        .await
        .unwrap()
        .first()
        .map(|p| p.id().clone());
    fixture
        .usecase()
        .execute(CreateProjectFromYoutubeRequest {
            url: URL.into(),
            project_id,
        })
        .await
        .unwrap();
    let outbox = SqliteOutboxRepository::new(fixture.pool.clone());
    for message in outbox.fetch_pending(10).await.unwrap().messages {
        if let OutboxPayload::FinalizeStagedArtifact {
            staging_key,
            final_key,
            ..
        } = message.payload
        {
            assert!(
                outbox
                    .mark_processing(&message.id, "crash-child")
                    .await
                    .unwrap()
            );
            fixture
                .store
                .finalize_staged_artifact(&staging_key, &final_key)
                .await
                .unwrap();
            checkpoint(&fixture.root, "outbox-before-ack");
        }
    }
    panic!("crash checkpoint not reached");
}

#[tokio::test]
async fn youtube_resume_recovers_after_real_process_kills_without_partial_projects_or_duplicates() {
    for existing in [false, true] {
        for phase in [
            "download",
            "stage-before-checkpoint",
            "stage-after-checkpoint",
            "before-commit",
            "after-commit",
            "outbox-before-ack",
        ] {
            let root = tempfile::tempdir().unwrap();
            let fixture = Fixture::open(root.path()).await;
            let original = if existing {
                Some(
                    fixture
                        .repo
                        .create(Project::new("Keep title".into()).unwrap())
                        .await
                        .unwrap(),
                )
            } else {
                None
            };
            fixture.pool.close().await;
            kill_at(root.path(), phase);

            let fixture = Fixture::open(root.path()).await;
            let pending = fixture.journal.list().await.unwrap();
            let committed = matches!(phase, "after-commit" | "outbox-before-ack");
            assert_eq!(pending.len(), usize::from(!committed), "{phase}");
            if !committed {
                assert_eq!(
                    fixture.repo.list().await.unwrap(),
                    original.clone().into_iter().collect::<Vec<_>>()
                );
                let id = pending[0].project.id.clone();
                let lease = fixture.workspace.acquire_import_lease(&id).await.unwrap();
                assert!(fixture.usecase().resume(&id).await.is_err());
                drop(lease);
                let resumed = fixture.usecase().resume(&id).await.unwrap();
                assert_eq!(resumed.project.id(), &id);
            }
            assert!(fixture.journal.list().await.unwrap().is_empty());
            let projects = fixture.repo.list().await.unwrap();
            assert_eq!(projects.len(), 1, "{phase}");
            assert_eq!(projects[0].status(), &ProjectStatus::ReadyForProcessing);
            assert_eq!(projects[0].revision(), if existing { 2 } else { 1 });
            if existing {
                assert_eq!(projects[0].title(), "Keep title");
            }
            let index = Arc::new(SqliteArtifactIndex::new(fixture.pool.clone()));
            assert!(
                index
                    .list_by_project(projects[0].id())
                    .await
                    .unwrap()
                    .is_empty()
            );
            let outbox = SqliteOutboxRepository::new(fixture.pool.clone());
            if phase == "outbox-before-ack" {
                assert_eq!(outbox.fetch_pending(10).await.unwrap().messages.len(), 1);
                sqlx::query("UPDATE outbox_messages SET locked_at = '2000-01-01T00:00:00Z' WHERE status = 'processing'").execute(&fixture.pool).await.unwrap();
            }
            let worker = OutboxWorker::new(
                outbox.clone(),
                fixture.store.clone(),
                index.clone(),
                Arc::new(SqliteStorageUnitOfWork::new(fixture.pool.clone())),
                Arc::new(UnusedEventPublisher),
                fixture.workspace.clone(),
                OutboxMaintenanceConfig::try_default().unwrap(),
            );
            let report = worker.process_pending_messages(10).await.unwrap();
            assert_eq!(report.completed, 2, "{phase}: {report:?}");
            assert_eq!(
                worker.process_pending_messages(10).await.unwrap().fetched,
                0
            );
            let artifacts = index.list_by_project(projects[0].id()).await.unwrap();
            assert_eq!(artifacts.len(), 1);
            assert_eq!(artifacts[0].state, ArtifactState::Ready);
            assert_eq!(
                std::fs::read(fixture.store.resolve_artifact(&artifacts[0]).await.unwrap())
                    .unwrap(),
                b"partial video"
            );
            assert_eq!(
                std::fs::read(root.path().join("network-completions")).unwrap(),
                b"1"
            );
            assert_eq!(
                sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
                    .fetch_one(&fixture.pool)
                    .await
                    .unwrap(),
                "ok"
            );
            let dirs = root.path().join("tmp").join(projects[0].id().to_string());
            assert_eq!(std::fs::read_dir(dirs).unwrap().count(), 0);
            fixture.pool.close().await;
        }
    }
}
