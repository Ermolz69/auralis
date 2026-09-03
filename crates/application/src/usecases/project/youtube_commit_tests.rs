#![allow(clippy::unwrap_used)]

use super::youtube_atomic_support::Fixture;
use crate::usecases::media::download_youtube_video::{
    DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase,
};
use adapters_storage::sqlite::SqliteStorageUnitOfWork;
use domain::{media::MediaSource, project::Project};
use ports::workspace::TempWorkspacePort;
use ports::{
    error::PortError,
    project_update::ProjectUpdate,
    repository::ProjectRepository,
    source::VideoSourcePort,
    transaction::{CommitYoutubeImport, StorageUnitOfWork},
};

async fn prepare(fixture: &Fixture, mut project: Project, existing: bool) -> CommitYoutubeImport {
    let source = MediaSource::YoutubeUrl {
        url: "https://youtube.com/watch?v=fence".into(),
    };
    let original_updated_at = existing.then_some(project.updated_at());
    let metadata = fixture.source.fetch_metadata(&source).await.unwrap();
    project
        .import_source(source.clone(), Some(metadata))
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    let allocation = fixture
        .workspace
        .create_allocation(project.id(), "test")
        .await
        .unwrap();
    let write = DownloadYoutubeVideoUseCase::new(fixture.source.clone(), fixture.store.clone())
        .execute(DownloadYoutubeVideoRequest {
            project_id: project.id().clone(),
            source,
            temp_dir: allocation.absolute_path,
            workspace_key: allocation.workspace_key,
            filename_hint: Some("original".into()),
        })
        .await
        .unwrap();
    CommitYoutubeImport {
        project,
        write,
        original_updated_at,
    }
}

async fn assert_no_artifact_writes(fixture: &Fixture) {
    for table in ["artifacts", "outbox_messages"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&fixture.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[tokio::test]
async fn transaction_rechecks_revision_status_and_active_job_even_after_application_validation() {
    for change in ["revision", "status", "active_job", "deleted"] {
        let fixture = Fixture::new().await;
        let original = fixture
            .repo
            .create(Project::new("Original".into()).unwrap())
            .await
            .unwrap();
        let command = prepare(&fixture, original.clone(), true).await;
        match change {
            "revision" => {
                fixture
                    .repo
                    .update(
                        original.id(),
                        original.revision(),
                        ProjectUpdate::Rename {
                            title: "Concurrent".into(),
                        },
                        original.updated_at(),
                    )
                    .await
                    .unwrap();
            }
            "status" => {
                sqlx::query("UPDATE projects SET status = 'Cancelled' WHERE id = ?")
                    .bind(original.id().to_string())
                    .execute(&fixture.pool)
                    .await
                    .unwrap();
            }
            "active_job" => {
                sqlx::query("UPDATE projects SET active_job_id = ? WHERE id = ?")
                    .bind(domain::job::JobId::new().to_string())
                    .bind(original.id().to_string())
                    .execute(&fixture.pool)
                    .await
                    .unwrap();
            }
            _ => fixture.repo.delete(original.id()).await.unwrap(),
        }
        let current = fixture.repo.get(original.id()).await.unwrap();
        let result = SqliteStorageUnitOfWork::new(fixture.pool.clone())
            .commit_youtube_import(command)
            .await;
        if change == "deleted" {
            assert!(matches!(result, Err(PortError::NotFound { .. })));
        } else {
            assert!(matches!(result, Err(PortError::Conflict { .. })));
        }
        assert_eq!(fixture.repo.get(original.id()).await.unwrap(), current);
        assert_no_artifact_writes(&fixture).await;
        fixture.pool.close().await;
    }
}

#[tokio::test]
async fn a_new_project_id_collision_does_not_overwrite_the_existing_project() {
    let fixture = Fixture::new().await;
    let original = fixture
        .repo
        .create(Project::new("Original".into()).unwrap())
        .await
        .unwrap();
    let command = prepare(&fixture, original.clone(), false).await;
    let result = SqliteStorageUnitOfWork::new(fixture.pool.clone())
        .commit_youtube_import(command)
        .await;
    assert!(matches!(result, Err(PortError::Conflict { .. })));
    assert_eq!(
        fixture.repo.get(original.id()).await.unwrap(),
        Some(original)
    );
    assert_no_artifact_writes(&fixture).await;
    fixture.pool.close().await;
}

#[tokio::test]
async fn atomic_command_rejects_inconsistent_ownership_and_artifact_state_before_writes() {
    let fixture = Fixture::new().await;
    let uow = SqliteStorageUnitOfWork::new(fixture.pool.clone());
    for invalid in [
        "workspace",
        "artifact_state",
        "artifact_kind",
        "project",
        "source",
        "staging",
        "location",
        "revision",
    ] {
        let mut command = prepare(&fixture, Project::new("Candidate".into()).unwrap(), false).await;
        match invalid {
            "workspace" => {
                command.write.temp_workspace_key =
                    Some(domain::outbox::WorkspaceKey::new("tmp/foreign/allocation").unwrap())
            }
            "artifact_state" => command.write.artifact.state = domain::media::ArtifactState::Ready,
            "artifact_kind" => {
                command.write.artifact.kind = domain::media::ArtifactKind::SourceVideo
            }
            "project" => command.write.project_id = domain::project::ProjectId::new(),
            "source" => command.project = Project::new("Draft".into()).unwrap(),
            "staging" => command.write.staging_key = "../escape".into(),
            "location" => {
                command.write.artifact.location =
                    domain::media::ArtifactLocation::LocalPath("original.mp4".into())
            }
            _ => command.project.advance_revision().unwrap(),
        }
        assert!(matches!(
            uow.commit_youtube_import(command).await,
            Err(PortError::Unexpected { .. })
        ));
        assert!(fixture.repo.list().await.unwrap().is_empty());
        assert_no_artifact_writes(&fixture).await;
    }
    fixture.pool.close().await;
}
