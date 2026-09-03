use adapters_storage::{
    local::LocalTempWorkspace,
    sqlite::{SqliteProjectRepository, SqliteStorageUnitOfWork, connect_sqlite},
};
use adapters_ytdlp::mock::MockVideoSourceAdapter;
use domain::{
    job::{Job, JobKind},
    media::MediaSource,
    project::{Project, ProjectStatus},
};
use ports::{
    error::PortError,
    project_update::ProjectUpdate,
    repository::ProjectRepository,
    transaction::{CommitPipelineStart, StorageUnitOfWork},
};
use std::sync::Arc;

use super::{
    concurrency_support::PausedProjectRepository,
    create_from_youtube::{CreateProjectFromYoutubeRequest, CreateProjectFromYoutubeUseCase},
    import_source::{ImportVideoSourceRequest, ImportVideoSourceUseCase},
    rename::{RenameProjectRequest, RenameProjectUseCase},
};
use crate::{
    error::ApplicationError,
    test_utils::{MockArtifactStore, MockStorageUnitOfWork},
};

#[tokio::test]
async fn rename_conflicts_with_pipeline_then_retry_only_changes_title() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("projects.sqlite"))
        .await
        .unwrap();
    let repo = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let uow = SqliteStorageUnitOfWork::new(pool);
    let mut project = Project::new("Original".into());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=original".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    repo.create(project.clone()).await.unwrap();
    let paused = PausedProjectRepository::new(repo.clone(), false);
    let use_case = RenameProjectUseCase::new(paused.clone());
    let request = RenameProjectRequest {
        project_id: project.id().clone(),
        title: "New title".into(),
    };
    let rename = tokio::spawn(async move { use_case.execute(request).await });
    paused.wait_for_write().await;

    let job = Job::new(
        project.id().clone(),
        project.title().into(),
        JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    uow.commit_pipeline_start(CommitPipelineStart {
        project: project.clone(),
        job,
    })
    .await
    .unwrap();
    let processing = repo.get(project.id()).await.unwrap().unwrap();
    assert_eq!(processing.revision(), project.revision() + 1);
    paused.release();
    assert!(matches!(
        rename.await.unwrap(),
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), processing);

    let renamed = RenameProjectUseCase::new(repo.clone())
        .execute(RenameProjectRequest {
            project_id: project.id().clone(),
            title: "  New title  ".into(),
        })
        .await
        .unwrap();
    let mut expected = processing.to_snapshot();
    expected.title = "New title".into();
    expected.updated_at = renamed.updated_at();
    expected.revision += 1;
    assert_eq!(renamed.to_snapshot(), expected);
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), renamed);
}

#[tokio::test]
async fn source_import_conflicts_without_overwriting_a_competing_import() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("projects.sqlite"))
        .await
        .unwrap();
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let project = repo.create(Project::new("Original".into())).await.unwrap();
    let paused = PausedProjectRepository::new(repo.clone(), false);
    let use_case = ImportVideoSourceUseCase::new(paused.clone(), MockVideoSourceAdapter::new());
    let request = ImportVideoSourceRequest {
        project_id: project.id().clone(),
        source: MediaSource::YoutubeUrl {
            url: "https://youtube.com/watch?v=stale".into(),
        },
    };
    let import = tokio::spawn(async move { use_case.execute(request).await });
    paused.wait_for_write().await;

    let current = ImportVideoSourceUseCase::new(repo.clone(), MockVideoSourceAdapter::new())
        .execute(ImportVideoSourceRequest {
            project_id: project.id().clone(),
            source: MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=current".into(),
            },
        })
        .await
        .unwrap()
        .project;
    paused.release();
    assert!(matches!(
        import.await.unwrap(),
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), current);
    assert_eq!(current.revision(), project.revision() + 1);
}

#[tokio::test]
async fn youtube_finalization_conflicts_without_reverting_a_rename_during_download() {
    let dir = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(dir.path().join("projects.sqlite"))
        .await
        .unwrap();
    let repo = Arc::new(SqliteProjectRepository::new(pool));
    let project = repo.create(Project::new("Original".into())).await.unwrap();
    let paused = PausedProjectRepository::new(repo.clone(), true);
    let use_case = CreateProjectFromYoutubeUseCase::new(
        paused.clone(),
        MockVideoSourceAdapter::new(),
        MockArtifactStore,
        MockStorageUnitOfWork::new(),
        Arc::new(LocalTempWorkspace::new(dir.path().to_path_buf())),
    );
    let request = CreateProjectFromYoutubeRequest {
        project_id: Some(project.id().clone()),
        url: "https://youtube.com/watch?v=download".into(),
    };
    let create = tokio::spawn(async move { use_case.execute(request).await });
    paused.wait_for_write().await;

    let renamed = RenameProjectUseCase::new(repo.clone())
        .execute(RenameProjectRequest {
            project_id: project.id().clone(),
            title: "User title".into(),
        })
        .await
        .unwrap();
    paused.release();
    assert!(matches!(
        create.await.unwrap(),
        Err(ApplicationError::Port(PortError::Conflict { .. }))
    ));
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), renamed);
    assert_eq!(renamed.status(), &ProjectStatus::SourceImported);
    assert!(renamed.metadata().is_some());

    let ready = repo
        .update(
            project.id(),
            renamed.revision(),
            ProjectUpdate::MarkReadyForProcessing,
            chrono::Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(ready.title(), "User title");
    assert_eq!(ready.status(), &ProjectStatus::ReadyForProcessing);
    assert_eq!(ready.metadata(), renamed.metadata());
}
