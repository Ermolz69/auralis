use super::{
    create_from_youtube::CreateProjectFromYoutubeRequest, youtube_atomic_support::Fixture,
};
use crate::error::ApplicationError;
use domain::project::{Project, ProjectStatus};
use ports::{error::PortError, repository::ProjectRepository};
use std::sync::atomic::Ordering;

#[tokio::test]
async fn a_competing_import_wins_without_being_overwritten_by_the_slow_download() {
    let fixture = Fixture::new().await;
    let original = fixture
        .repo
        .create(Project::new("Original".into()))
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
    let winner = fixture
        .usecase()
        .execute(CreateProjectFromYoutubeRequest {
            url: "https://youtube.com/watch?v=winner".into(),
            project_id: Some(original.id().clone()),
        })
        .await
        .unwrap()
        .project;
    fixture.source.released.notify_one();
    assert!(matches!(
        slow.await.unwrap(),
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
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
    assert!(!paths[0].parent().unwrap().exists());
    assert!(paths[1].parent().unwrap().exists());
    fixture.pool.close().await;
}
