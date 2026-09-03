#![allow(clippy::unwrap_used)]

use super::{
    create_from_youtube::CreateProjectFromYoutubeRequest, youtube_atomic_support::Fixture,
};
use crate::error::ApplicationError;
use domain::project::{Project, ProjectStatus};
use ports::{error::PortError, repository::ProjectRepository};
use std::sync::atomic::Ordering;

#[tokio::test]
async fn project_deletion_cancels_a_running_download_before_cleanup_retries_are_exhausted() {
    use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};
    let fixture = Fixture::new().await;
    let project = fixture
        .repo
        .create(Project::new("Delete running".into()).unwrap())
        .await
        .unwrap();
    fixture.source.failure.store(5, Ordering::SeqCst);
    let usecase = fixture.usecase();
    let id = project.id().clone();
    let task = tokio::spawn(async move {
        usecase
            .execute(CreateProjectFromYoutubeRequest {
                url: "https://youtube.com/watch?v=cancel".into(),
                project_id: Some(id),
            })
            .await
    });
    fixture.source.wait().await;
    adapters_storage::sqlite::SqliteStorageUnitOfWork::new(fixture.pool.clone())
        .commit_project_delete(CommitProjectDelete {
            project_id: project.id().clone(),
        })
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(fixture.usecase().list_pending().await.unwrap().is_empty());
    assert!(fixture.repo.get(project.id()).await.unwrap().is_none());
    fixture.pool.close().await;
}

#[tokio::test]
async fn a_pending_import_rejects_competing_sources_without_overwriting_either_project() {
    let fixture = Fixture::new().await;
    let original = fixture
        .repo
        .create(Project::new("Original".into()).unwrap())
        .await
        .unwrap();
    fixture.source.failure.store(5, Ordering::SeqCst);
    let usecase = fixture.usecase();
    let request = CreateProjectFromYoutubeRequest {
        url: "https://youtube.com/watch?v=slow".into(),
        project_id: Some(original.id().clone()),
    };
    let slow = tokio::spawn(async move { usecase.execute(request).await });
    fixture.source.wait().await;
    assert_eq!(
        fixture.repo.get(original.id()).await.unwrap(),
        Some(original.clone())
    );
    fixture.source.failure.store(0, Ordering::SeqCst);
    let competing = fixture
        .usecase()
        .execute(CreateProjectFromYoutubeRequest {
            url: "https://youtube.com/watch?v=winner".into(),
            project_id: Some(original.id().clone()),
        })
        .await;
    assert!(matches!(
        competing,
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    assert!(fixture.usecase().discard(original.id()).await.is_err());
    fixture.source.released.notify_one();
    let winner = slow.await.unwrap().unwrap().project;
    assert_eq!(winner.status(), &ProjectStatus::ReadyForProcessing);
    assert_eq!(fixture.repo.get(original.id()).await.unwrap(), Some(winner));
    let artifact_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
        .fetch_one(&fixture.pool)
        .await
        .unwrap();
    assert_eq!(artifact_count, 1);
    assert_eq!(outbox_count, 2);
    let paths = fixture.source.paths.lock().unwrap().clone();
    assert_eq!(paths.len(), 1);
    assert!(paths[0].parent().unwrap().exists());
    fixture.pool.close().await;
}
