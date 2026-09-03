use super::youtube_cleanup::cleanup_failed_import;
use crate::error::ApplicationError;
use crate::usecases::transcript::import_youtube_subtitles::mocks::{
    MockArtifactStoreForSubs, MockWorkspacePortForSubs,
};
use domain::outbox::WorkspaceKey;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn cleanup_attempts_both_owned_resources_and_preserves_the_primary_failure() {
    for fail_stage in [false, true] {
        for fail_workspace in [false, true] {
            let deleted_staging = Arc::new(Mutex::new(Vec::new()));
            let deleted_workspace = Arc::new(Mutex::new(Vec::new()));
            let store = MockArtifactStoreForSubs {
                fail_delete: fail_stage,
                deleted_keys: deleted_staging.clone(),
            };
            let workspace = MockWorkspacePortForSubs {
                fail_delete: fail_workspace,
                deleted_keys: deleted_workspace.clone(),
                allocated_path: Arc::default(),
            };
            let key = WorkspaceKey::new("tmp/project/allocation").unwrap();
            let error = cleanup_failed_import(
                ApplicationError::InvalidOperation {
                    message: "primary failure".into(),
                },
                Some(".staging/owned/file.mp4"),
                &key,
                &store,
                &workspace,
            )
            .await;
            assert_eq!(
                deleted_staging.lock().unwrap().len(),
                usize::from(!fail_stage)
            );
            assert_eq!(
                deleted_workspace.lock().unwrap().len(),
                usize::from(!fail_workspace)
            );
            if fail_stage || fail_workspace {
                let ApplicationError::OperationFailedWithCleanup {
                    primary,
                    cleanup_report,
                } = error
                else {
                    panic!("cleanup failure must be reported")
                };
                assert!(
                    matches!(*primary, ApplicationError::InvalidOperation { ref message } if message == "primary failure")
                );
                assert_eq!(
                    cleanup_report.staging_failure_count(),
                    usize::from(fail_stage)
                );
                assert_eq!(
                    cleanup_report.workspace_failure_count(),
                    usize::from(fail_workspace)
                );
            } else {
                assert!(
                    matches!(error, ApplicationError::InvalidOperation { ref message } if message == "primary failure")
                );
            }
        }
    }
}
