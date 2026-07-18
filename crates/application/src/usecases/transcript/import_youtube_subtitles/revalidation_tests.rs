#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::mocks::{
    MockArtifactStoreForSubs, MockProjectRepo, MockSubtitleSource, MockWorkspacePortForSubs,
};
use super::*;
use crate::{error::ApplicationError, test_utils::MockStorageUnitOfWork};
use domain::{media::MediaSource, project::Project};
use ports::error::PortError;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_workspace_error_composites_with_primary_error() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let job_id = domain::job::JobId::new();
    project.mark_ready_for_processing().unwrap();
    project.start_processing(job_id.clone()).unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: true,
            deleted_keys: deleted_staging.clone(),
        }), // staging delete fails
        Arc::new(MockStorageUnitOfWork::with_failure()), // Will fail commit
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: true,
            deleted_keys: deleted_workspace.clone(),
            allocated_path: Arc::new(Mutex::new(None)),
        }), // workspace delete fails
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            job_id,
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };

    if let ApplicationError::OperationFailedWithCleanup {
        primary,
        cleanup_report,
    } = err
    {
        assert!(matches!(*primary, ApplicationError::Port(_))); // The initial transaction failure
        assert_eq!(cleanup_report.failures.len(), 2);

        // Ensure both failures are reported
        let has_workspace_err = cleanup_report
            .failures
            .iter()
            .any(|f| matches!(f.target, crate::error::CleanupTarget::Workspace { .. }));
        let has_staging_err = cleanup_report
            .failures
            .iter()
            .any(|f| matches!(f.target, crate::error::CleanupTarget::Staging { .. }));
        assert!(has_workspace_err);
        assert!(has_staging_err);
    } else {
        panic!("Expected OperationFailedWithCleanup, got {:?}", err);
    }
}

#[tokio::test]
async fn test_cancellation_during_download() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let job_id = domain::job::JobId::new();
    project.mark_ready_for_processing().unwrap();
    project.start_processing(job_id.clone()).unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let cancel_token = tokio_util::sync::CancellationToken::new();
    cancel_token.cancel(); // Cancel immediately

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: false,
            deleted_keys: deleted_staging.clone(),
        }),
        Arc::new(MockStorageUnitOfWork::new()),
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: false,
            deleted_keys: deleted_workspace.clone(),
            allocated_path: Arc::new(Mutex::new(None)),
        }),
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
            cancellation_token: cancel_token,
            job_id,
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };

    // Should fail with Cancelled error wrapped or plain
    let is_cancelled = match &err {
        ApplicationError::Port(PortError::Cancelled) => true,
        ApplicationError::OperationFailedWithCleanup { primary, .. } => {
            matches!(
                primary.as_ref(),
                ApplicationError::Port(PortError::Cancelled)
            )
        }
        _ => false,
    };
    assert!(is_cancelled, "Expected Cancelled, got {:?}", err);

    // Workspace should be deleted
    assert_eq!(deleted_workspace.lock().unwrap().len(), 1);
    assert_eq!(deleted_workspace.lock().unwrap()[0], "tmp/1/subs");
}

#[tokio::test]
async fn test_revalidation_job_id_mismatch_fails() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let job_id = domain::job::JobId::new();
    let mismatched_job_id = domain::job::JobId::new();
    project.mark_ready_for_processing().unwrap();
    project.start_processing(job_id.clone()).unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: false,
            deleted_keys: deleted_staging.clone(),
        }),
        Arc::new(MockStorageUnitOfWork::new()),
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: false,
            deleted_keys: deleted_workspace.clone(),
            allocated_path: Arc::new(Mutex::new(None)),
        }),
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            job_id: mismatched_job_id, // Pass incorrect job_id
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };

    let is_mismatch_err = match &err {
        ApplicationError::InvalidOperation { message } => message.to_lowercase().contains("job id"),
        ApplicationError::OperationFailedWithCleanup { primary, .. } => {
            if let ApplicationError::InvalidOperation { message } = primary.as_ref() {
                message.to_lowercase().contains("job id")
            } else {
                false
            }
        }
        _ => false,
    };
    assert!(is_mismatch_err, "Expected Job ID mismatch, got {:?}", err);
}

#[tokio::test]
async fn test_revalidation_status_mismatch_fails() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let job_id = domain::job::JobId::new();
    project.mark_ready_for_processing().unwrap();
    project.start_processing(job_id.clone()).unwrap();
    project
        .apply_terminal_transition(&job_id, domain::job::TerminalOutcome::Failed)
        .unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: false,
            deleted_keys: deleted_staging.clone(),
        }),
        Arc::new(MockStorageUnitOfWork::new()),
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: false,
            deleted_keys: deleted_workspace.clone(),
            allocated_path: Arc::new(Mutex::new(None)),
        }),
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            job_id,
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };

    let is_status_err = match &err {
        ApplicationError::InvalidOperation { message } => message.contains("status"),
        ApplicationError::OperationFailedWithCleanup { primary, .. } => {
            if let ApplicationError::InvalidOperation { message } = primary.as_ref() {
                message.contains("status")
            } else {
                false
            }
        }
        _ => false,
    };
    assert!(
        is_status_err,
        "Expected status Processing check, got {:?}",
        err
    );
}

#[test]
fn test_architecture_boundary_rejection() {
    let content = include_str!("usecase.rs");

    // Check that we don't have direct filesystem access in usecase.rs
    assert!(!content.contains("std::fs::"));
    assert!(!content.contains("tokio::fs::"));
    assert!(!content.contains("async_std::fs::"));
    // Verify no PathBuf leaked in staging signatures
    assert!(!content.contains(".stage_owned_temp_file("));
}
