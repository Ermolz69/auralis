use super::{
    SqliteProjectAvatarRepository, SqliteProjectRepository, SqliteStorageUnitOfWork, connect_sqlite,
};
use domain::project::{Project, avatar::ProjectAvatar};
use ports::{
    error::PortError,
    project_avatar::ProjectAvatarRepository,
    repository::ProjectRepository,
    transaction::{CommitPipelineStart, CommitProjectDelete, StorageUnitOfWork},
};

fn avatar() -> ProjectAvatar {
    ProjectAvatar::new("data:image/png;base64,iVBORw0KGgo=".into()).unwrap()
}

#[tokio::test]
async fn avatar_survives_project_updates_and_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("avatars.sqlite");
    let pool = connect_sqlite(&path).await.unwrap();
    let projects = SqliteProjectRepository::new(pool.clone());
    let avatars = SqliteProjectAvatarRepository::new(pool.clone());
    let project = Project::new("Original".into());
    projects.create(project.clone()).await.unwrap();
    assert!(!avatars.get(project.id()).await.unwrap().initialized);
    avatars
        .set(project.id(), Some(avatar()), false)
        .await
        .unwrap();
    assert_eq!(projects.get(project.id()).await.unwrap().unwrap(), project);
    projects
        .update(
            project.id(),
            project.revision(),
            ports::project_update::ProjectUpdate::Rename {
                title: "Renamed".into(),
            },
            project.updated_at(),
        )
        .await
        .unwrap();
    assert_eq!(
        avatars.get(project.id()).await.unwrap().avatar,
        Some(avatar())
    );
    pool.close().await;
    let pool = connect_sqlite(&path).await.unwrap();
    assert_eq!(
        SqliteProjectAvatarRepository::new(pool)
            .get(project.id())
            .await
            .unwrap()
            .avatar,
        Some(avatar())
    );
}

#[tokio::test]
async fn stale_legacy_migration_cannot_replace_saved_or_removed_avatar() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(&dir.path().join("avatars.sqlite"))
        .await
        .unwrap();
    let projects = SqliteProjectRepository::new(pool.clone());
    let avatars = SqliteProjectAvatarRepository::new(pool);
    let project = Project::new("Project".into());
    projects.create(project.clone()).await.unwrap();
    let saved = avatars
        .set(project.id(), Some(avatar()), true)
        .await
        .unwrap();
    assert!(saved.initialized);
    let other = ProjectAvatar::new("data:image/gif;base64,R0lGODlh".into()).unwrap();
    assert_eq!(
        avatars
            .set(project.id(), Some(other), true)
            .await
            .unwrap()
            .avatar,
        Some(avatar())
    );
    avatars.set(project.id(), None, false).await.unwrap();
    let after = avatars
        .set(project.id(), Some(avatar()), true)
        .await
        .unwrap();
    assert!(after.initialized);
    assert_eq!(after.avatar, None);
}

#[tokio::test]
async fn deletion_removes_avatar_and_late_writes_cannot_recreate_project() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(&dir.path().join("avatars.sqlite"))
        .await
        .unwrap();
    let projects = SqliteProjectRepository::new(pool.clone());
    let avatars = SqliteProjectAvatarRepository::new(pool.clone());
    let project = Project::new("Project".into());
    projects.create(project.clone()).await.unwrap();
    avatars
        .set(project.id(), Some(avatar()), false)
        .await
        .unwrap();
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    sqlx::raw_sql("CREATE TRIGGER reject_cleanup BEFORE INSERT ON outbox_messages BEGIN SELECT RAISE(ABORT, 'injected'); END;").execute(&pool).await.unwrap();
    assert!(
        uow.commit_project_delete(CommitProjectDelete {
            project_id: project.id().clone()
        })
        .await
        .is_err()
    );
    assert_eq!(
        avatars.get(project.id()).await.unwrap().avatar,
        Some(avatar())
    );
    sqlx::query("DROP TRIGGER reject_cleanup")
        .execute(&pool)
        .await
        .unwrap();
    uow.commit_project_delete(CommitProjectDelete {
        project_id: project.id().clone(),
    })
    .await
    .unwrap();
    assert!(matches!(
        avatars.get(project.id()).await,
        Err(PortError::NotFound { .. })
    ));
    assert!(matches!(
        avatars.set(project.id(), Some(avatar()), false).await,
        Err(PortError::NotFound { .. })
    ));
    assert!(projects.get(project.id()).await.unwrap().is_none());
}

#[tokio::test]
async fn aggregate_pipeline_commit_preserves_avatar_written_after_snapshot_read() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("pipeline.sqlite"))
        .await
        .unwrap();
    let projects = SqliteProjectRepository::new(pool.clone());
    let avatars = SqliteProjectAvatarRepository::new(pool.clone());
    let mut project = Project::new("Project".into());
    project
        .import_source(
            domain::media::MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=current".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    let mut snapshot = projects.create(project).await.unwrap();
    avatars
        .set(snapshot.id(), Some(avatar()), false)
        .await
        .unwrap();
    let job = domain::job::Job::new(
        snapshot.id().clone(),
        snapshot.title().into(),
        domain::job::JobKind::Dubbing,
    );
    snapshot.start_processing(job.id().clone()).unwrap();
    SqliteStorageUnitOfWork::new(pool)
        .commit_pipeline_start(CommitPipelineStart {
            project: snapshot.clone(),
            job,
        })
        .await
        .unwrap();
    assert_eq!(
        avatars.get(snapshot.id()).await.unwrap().avatar,
        Some(avatar())
    );
}

#[tokio::test]
async fn database_enforces_size_and_reports_corrupt_avatars_without_the_payload() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(&dir.path().join("avatars.sqlite"))
        .await
        .unwrap();
    let project = Project::new("Project".into());
    SqliteProjectRepository::new(pool.clone())
        .create(project.clone())
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE projects SET avatar_data_url = ?")
            .bind("x".repeat(1_398_201))
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("UPDATE projects SET avatar_data_url = 'private-invalid-payload'")
        .execute(&pool)
        .await
        .unwrap();
    let error = SqliteProjectAvatarRepository::new(pool)
        .get(project.id())
        .await
        .unwrap_err();
    assert!(matches!(error, PortError::InvalidStoredData { .. }));
    assert!(!error.to_string().contains("private-invalid-payload"));
}
