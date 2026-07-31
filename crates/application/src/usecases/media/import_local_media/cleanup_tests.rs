#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;

use domain::project::Project;
use ports::repository::ProjectRepository;

use super::tests::{MockArtifactStore, MockProbe, create_temp_file};
use super::*;
use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use crate::test_utils::MockStorageUnitOfWork;
use crate::usecases::project::lifecycle::ProjectLifecycleLocks;
use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};

#[tokio::test]
async fn test_commit_fail_cleanup_success() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Commit Fail".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::with_failure());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Port(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_commit_fail_cleanup_fail() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Double Fail".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::with_failure());
    let store = MockArtifactStore::new();
    store.set_should_fail_delete(true);
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store, locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    match err {
        ApplicationError::OperationFailedWithCleanup {
            primary,
            cleanup_report,
        } => {
            assert!(matches!(*primary, ApplicationError::Port(_)));
            assert_eq!(cleanup_report.staging_failure_count(), 1);
        }
        _ => panic!("Expected OperationFailedWithCleanup, got {:?}", err),
    }
}

#[tokio::test]
async fn test_command_validation_failure_triggers_cleanup() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Validation Fail".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::with_failure());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Port(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}

#[test]
fn test_cleanup_formatting_sanitizes_application_error() {
    let mut report = CleanupReport::new();
    report.add_failure(
        CleanupTarget::staging(".staging/secret-key"),
        ports::error::PortError::Io {
            message: "Failed to delete file C:\\secret\\video.mp4".into(),
        },
    );

    let app_err = ApplicationError::OperationFailedWithCleanup {
        primary: Box::new(ApplicationError::Unexpected("Primary err".into())),
        cleanup_report: report,
    };

    let display_str = format!("{}", app_err);
    let debug_str = format!("{:?}", app_err);

    assert!(!display_str.contains("C:\\secret\\video.mp4"));
    assert!(!display_str.contains(".staging/secret-key"));
    assert!(!debug_str.contains("C:\\secret\\video.mp4"));
    assert!(!debug_str.contains(".staging/secret-key"));

    assert!(display_str.contains("staging_failures") || display_str.contains("Staging"));
    assert!(debug_str.contains("staging_failures") || debug_str.contains("Staging"));
}

#[test]
fn test_cleanup_failure_formatting() {
    let mut report = CleanupReport::new();
    report.add_failure(
        CleanupTarget::staging(".staging/secret-key"),
        ports::error::PortError::Io {
            message: "Failed to delete file C:\\secret\\video.mp4".into(),
        },
    );

    let report_display = format!("{}", report);
    let report_debug = format!("{:?}", report);

    assert!(!report_display.contains("C:\\secret\\video.mp4"));
    assert!(!report_display.contains(".staging/secret-key"));
    assert!(!report_debug.contains("C:\\secret\\video.mp4"));
    assert!(!report_debug.contains(".staging/secret-key"));
}
