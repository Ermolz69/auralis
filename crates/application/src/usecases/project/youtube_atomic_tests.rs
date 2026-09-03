#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{
    create_from_youtube::CreateProjectFromYoutubeRequest, youtube_atomic_support::Fixture,
};
use crate::error::ApplicationError;
use domain::project::{Project, ProjectId, ProjectStatus};
use ports::{error::PortError, project_update::ProjectUpdate, repository::ProjectRepository};
use std::sync::atomic::Ordering;

fn request(project_id: Option<ProjectId>) -> CreateProjectFromYoutubeRequest {
    CreateProjectFromYoutubeRequest {
        url: "https://youtube.com/watch?v=atomic".into(),
        project_id,
    }
}

#[tokio::test]
async fn source_failures_leave_new_and_existing_projects_unchanged_and_retryable() {
    for existing in [false, true] {
        for phase in 1..=4 {
            let fixture = Fixture::new().await;
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
            let id = original.as_ref().map(|project| project.id().clone());
            fixture.source.failure.store(phase, Ordering::SeqCst);
            assert!(
                fixture
                    .usecase()
                    .execute(request(id.clone()))
                    .await
                    .is_err()
            );
            fixture.assert_unchanged(original.as_ref()).await;

            fixture.source.failure.store(0, Ordering::SeqCst);
            let response = fixture.usecase().execute(request(id)).await.unwrap();
            assert_eq!(
                response.project.status(),
                &ProjectStatus::ReadyForProcessing
            );
            assert!(response.project.metadata().is_some());
            assert!(response.project.transcript().is_none());
            assert_eq!(
                fixture
                    .repo
                    .get(response.project.id())
                    .await
                    .unwrap()
                    .unwrap(),
                response.project
            );
            assert_eq!(response.project.revision(), if existing { 2 } else { 1 });
            assert_eq!(fixture.repo.list().await.unwrap().len(), 1);
            if existing {
                assert_eq!(response.project.title(), "Keep title");
            }
            fixture.pool.close().await;
        }
    }
}

#[tokio::test]
async fn artifact_and_each_outbox_failure_roll_back_every_write_and_allow_retry() {
    for existing in [false, true] {
        for (table, condition) in [
            ("artifacts", "1"),
            ("outbox_messages", "NEW.kind = 'finalize_staged_artifact'"),
            (
                "outbox_messages",
                "NEW.kind = 'delete_workspace_allocation'",
            ),
        ] {
            let fixture = Fixture::new().await;
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
            let id = original.as_ref().map(|project| project.id().clone());
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE TRIGGER reject_import BEFORE INSERT ON {table} WHEN {condition} BEGIN SELECT RAISE(ABORT, 'injected failure'); END;"))).execute(&fixture.pool).await.unwrap();
            assert!(
                fixture
                    .usecase()
                    .execute(request(id.clone()))
                    .await
                    .is_err()
            );
            fixture.assert_unchanged(original.as_ref()).await;
            sqlx::raw_sql("DROP TRIGGER reject_import")
                .execute(&fixture.pool)
                .await
                .unwrap();
            fixture.usecase().execute(request(id)).await.unwrap();
            fixture.pool.close().await;
        }
    }
}

#[tokio::test]
async fn rename_during_download_is_preserved_and_the_import_can_be_retried() {
    let fixture = Fixture::new().await;
    let project = fixture
        .repo
        .create(Project::new("Original".into()).unwrap())
        .await
        .unwrap();
    fixture.source.failure.store(5, Ordering::SeqCst);
    let usecase = fixture.usecase();
    let input = request(Some(project.id().clone()));
    let download = tokio::spawn(async move { usecase.execute(input).await });
    fixture.source.wait().await;
    let lock = fixture.locks.get_lock(project.id()).unwrap();
    let guard = lock
        .try_lock()
        .expect("download must not hold the lifecycle lock");
    let renamed = fixture
        .repo
        .update(
            project.id(),
            project.revision(),
            ProjectUpdate::Rename {
                title: "User title".into(),
            },
            project.updated_at(),
        )
        .await
        .unwrap();
    drop(guard);
    fixture.source.released.notify_one();
    assert!(matches!(
        download.await.unwrap(),
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    fixture.assert_unchanged(Some(&renamed)).await;
    fixture.source.failure.store(0, Ordering::SeqCst);
    let response = fixture
        .usecase()
        .execute(request(Some(project.id().clone())))
        .await
        .unwrap();
    assert_eq!(response.project.title(), "User title");
    assert_eq!(response.project.revision(), renamed.revision() + 1);
    fixture.pool.close().await;
}

#[tokio::test]
async fn deleting_a_project_during_download_does_not_resurrect_it() {
    let fixture = Fixture::new().await;
    let project = fixture
        .repo
        .create(Project::new("Delete".into()).unwrap())
        .await
        .unwrap();
    fixture.source.failure.store(5, Ordering::SeqCst);
    let usecase = fixture.usecase();
    let input = request(Some(project.id().clone()));
    let download = tokio::spawn(async move { usecase.execute(input).await });
    fixture.source.wait().await;
    fixture.repo.delete(project.id()).await.unwrap();
    fixture.source.released.notify_one();
    assert!(matches!(
        download.await.unwrap(),
        Err(ApplicationError::ProjectNotFound(_))
    ));
    fixture.assert_unchanged(None).await;
    fixture.pool.close().await;
}

#[tokio::test]
async fn missing_or_non_draft_projects_fail_before_download() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        fixture
            .usecase()
            .execute(request(Some(ProjectId::new())))
            .await,
        Err(ApplicationError::ProjectNotFound(_))
    ));
    let created = fixture
        .usecase()
        .execute(request(None))
        .await
        .unwrap()
        .project;
    assert!(matches!(
        fixture
            .usecase()
            .execute(request(Some(created.id().clone())))
            .await,
        Err(ApplicationError::InvalidOperation { .. })
    ));
    assert_eq!(fixture.source.paths.lock().unwrap().len(), 1);
    assert_eq!(fixture.repo.get(created.id()).await.unwrap(), Some(created));
    fixture.pool.close().await;
}
