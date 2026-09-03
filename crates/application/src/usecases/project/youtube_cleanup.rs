use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use domain::outbox::WorkspaceKey;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

pub(super) async fn cleanup_failed_import(
    primary: ApplicationError,
    staging_key: Option<&str>,
    workspace_key: &WorkspaceKey,
    store: &impl ArtifactStore,
    workspace: &dyn TempWorkspacePort,
) -> ApplicationError {
    let mut report = CleanupReport::new();
    if let Some(key) = staging_key
        && let Err(error) = store.delete_storage_key(key).await
    {
        report.add_failure(CleanupTarget::staging(key), error);
    }
    if let Err(error) = workspace.delete_allocation(workspace_key).await {
        report.add_failure(CleanupTarget::workspace(workspace_key.as_str()), error);
    }
    if report.is_empty() {
        primary
    } else {
        ApplicationError::OperationFailedWithCleanup {
            primary: Box::new(primary),
            cleanup_report: report,
        }
    }
}
