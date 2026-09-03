#![allow(clippy::unwrap_used)]

use super::{
    delete::DeleteProjectRequest,
    open_folder::{OpenProjectFolderRequest, OpenProjectFolderUseCase},
    open_folder_support::{Fixture, Gate},
};
use crate::error::ApplicationError;
use ports::{error::PortError, repository::ProjectRepository};
use std::sync::{Arc, atomic::Ordering};

#[tokio::test]
async fn delete_wins_and_waiting_open_checks_existence_only_after_the_lock() {
    let fixture = Fixture::new().await;
    tokio::fs::create_dir_all(fixture.project_path())
        .await
        .unwrap();
    let gate = Arc::new(Gate::default());
    let delete = fixture.delete(Some(gate.clone()));
    let request = DeleteProjectRequest {
        project_id: fixture.project.id().clone(),
    };
    let deleting = tokio::spawn(async move { delete.execute(request).await });
    gate.wait_for_entry().await;
    fixture.process_cleanup().await;
    let repo = fixture.observed_repo(false);
    let workspace = fixture.workspace(None, false);
    let open =
        OpenProjectFolderUseCase::new(repo.clone(), workspace.clone(), fixture.locks.clone());
    let mut opening = Box::pin(open.execute(OpenProjectFolderRequest {
        project_id: fixture.project.id().clone(),
    }));
    assert!(futures::poll!(&mut opening).is_pending());
    assert_eq!(repo.reads.load(Ordering::SeqCst), 0);
    assert_eq!(workspace.calls.load(Ordering::SeqCst), 0);
    gate.released.notify_one();
    deleting.await.unwrap().unwrap();
    assert!(matches!(
        opening.await,
        Err(ApplicationError::ProjectNotFound(_))
    ));
    assert_eq!(repo.reads.load(Ordering::SeqCst), 1);
    assert_eq!(workspace.calls.load(Ordering::SeqCst), 0);
    assert!(!fixture.project_path().exists());
    fixture.pool.close().await;
}

#[tokio::test]
async fn open_wins_and_deletion_waits_until_the_workspace_operation_finishes() {
    let fixture = Fixture::new().await;
    let gate = Arc::new(Gate::default());
    let workspace = fixture.workspace(Some(gate.clone()), false);
    let open = OpenProjectFolderUseCase::new(
        fixture.repo.clone(),
        workspace.clone(),
        fixture.locks.clone(),
    );
    let request = OpenProjectFolderRequest {
        project_id: fixture.project.id().clone(),
    };
    let opening = tokio::spawn(async move { open.execute(request).await });
    gate.wait_for_entry().await;
    let lock = fixture.locks.get_lock(fixture.project.id()).unwrap();
    assert!(
        lock.try_lock().is_err(),
        "workspace operation must hold the lifecycle lock"
    );
    let delete = fixture.delete(None);
    let mut deleting = Box::pin(delete.execute(DeleteProjectRequest {
        project_id: fixture.project.id().clone(),
    }));
    assert!(futures::poll!(&mut deleting).is_pending());
    assert!(
        fixture
            .repo
            .get(fixture.project.id())
            .await
            .unwrap()
            .is_some()
    );
    gate.released.notify_one();
    opening.await.unwrap().unwrap();
    assert!(fixture.project_path().is_dir());
    deleting.await.unwrap();
    assert!(
        fixture
            .repo
            .get(fixture.project.id())
            .await
            .unwrap()
            .is_none()
    );
    fixture.process_cleanup().await;
    assert_eq!(workspace.calls.load(Ordering::SeqCst), 1);
    fixture.pool.close().await;
}

#[tokio::test]
async fn another_projects_lock_does_not_block_opening() {
    let fixture = Fixture::new().await;
    let other_lock = fixture
        .locks
        .get_lock(&domain::project::ProjectId::new())
        .unwrap();
    let _guard = other_lock.lock().await;
    let open = OpenProjectFolderUseCase::new(
        fixture.repo.clone(),
        fixture.workspace(None, false),
        fixture.locks.clone(),
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        open.execute(OpenProjectFolderRequest {
            project_id: fixture.project.id().clone(),
        }),
    )
    .await
    .unwrap()
    .unwrap();
    fixture.pool.close().await;
}

#[tokio::test]
async fn repository_and_workspace_failures_release_the_lifecycle_lock() {
    for repository_failure in [true, false] {
        let fixture = Fixture::new().await;
        let workspace = fixture.workspace(None, true);
        let open = OpenProjectFolderUseCase::new(
            fixture.observed_repo(repository_failure),
            workspace.clone(),
            fixture.locks.clone(),
        );
        let result = open
            .execute(OpenProjectFolderRequest {
                project_id: fixture.project.id().clone(),
            })
            .await;
        if repository_failure {
            assert!(matches!(
                result,
                Err(ApplicationError::Port(PortError::Storage { .. }))
            ));
            assert_eq!(workspace.calls.load(Ordering::SeqCst), 0);
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::Port(PortError::Io { .. }))
            ));
        }
        assert!(
            fixture
                .locks
                .get_lock(fixture.project.id())
                .unwrap()
                .try_lock()
                .is_ok()
        );
        fixture
            .delete(None)
            .execute(DeleteProjectRequest {
                project_id: fixture.project.id().clone(),
            })
            .await
            .unwrap();
        fixture.pool.close().await;
    }
}
