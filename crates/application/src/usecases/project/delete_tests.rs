#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use adapters_storage::sqlite::{
    SqliteJobRepository, SqliteProjectRepository, SqliteStorageUnitOfWork, connect_sqlite,
};
use domain::{
    job::{Job, JobKind},
    project::Project,
};
use jobs::manager::JobManager;
use ports::{
    job_runtime_control::{
        JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCompletion, RuntimeTask,
        RuntimeTaskOutcome,
    },
    repository::{JobRepository, ProjectRepository},
};

use super::{
    delete::{DeleteProjectRequest, DeleteProjectUseCase},
    lifecycle::ProjectLifecycleLocks,
};

#[tokio::test]
async fn deletion_commits_before_cleaning_up_an_already_completed_attached_task() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("project-delete.sqlite"))
        .await
        .unwrap();
    let project_repository = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let job_repository = Arc::new(SqliteJobRepository::new(pool.clone()));
    let storage_uow = Arc::new(SqliteStorageUnitOfWork::new(pool));
    let project = project_repository
        .create(Project::new("Delete with attached task".into()).unwrap())
        .await
        .unwrap();
    let job = job_repository
        .create(Job::new(
            project.id().clone(),
            "Attached task".into(),
            JobKind::Dubbing,
        ))
        .await
        .unwrap();
    let manager = Arc::new(JobManager::new(
        job_repository.clone(),
        storage_uow.clone(),
        None,
    ));

    manager
        .reserve(job.id().clone(), project.id().clone())
        .await
        .unwrap();
    let (cancel, _token) = ports::cancellation::CancelHandle::new();
    let join_handle = tokio::spawn(async { RuntimeTaskOutcome::Completed });
    tokio::task::yield_now().await;
    assert!(join_handle.is_finished());
    manager
        .attach_task(
            job.id().clone(),
            RuntimeTask {
                cancel,
                join_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    DeleteProjectUseCase::new(
        storage_uow,
        manager.clone(),
        Arc::new(ProjectLifecycleLocks::new()),
    )
    .execute(DeleteProjectRequest {
        project_id: project.id().clone(),
    })
    .await
    .unwrap();

    assert!(
        project_repository
            .get(project.id())
            .await
            .unwrap()
            .is_none()
    );
    assert!(job_repository.get(job.id()).await.unwrap().is_none());
    let repeated = manager
        .cancel_and_evict_jobs(std::slice::from_ref(job.id()))
        .await
        .unwrap();
    assert_eq!(
        repeated.jobs.get(job.id()),
        Some(&RuntimeCleanupOutcome::Missing)
    );
}
