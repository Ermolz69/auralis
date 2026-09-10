use crate::error::{CleanupReport, CleanupTarget};
use domain::outbox::WorkspaceKey;
use ports::{storage::ArtifactStore, workspace::TempWorkspacePort};

pub(super) async fn cleanup_staging(
    store: &dyn ArtifactStore,
    staging_key: &str,
    report: &mut CleanupReport,
) {
    if let Err(error) = store.delete_storage_key(staging_key).await {
        report.add_failure(CleanupTarget::staging(staging_key), error);
    }
}

pub(super) async fn cleanup_workspace(
    workspace: &dyn TempWorkspacePort,
    workspace_key: &WorkspaceKey,
    report: &mut CleanupReport,
) {
    if let Err(error) = workspace.delete_allocation(workspace_key).await {
        report.add_failure(CleanupTarget::workspace(workspace_key.as_str()), error);
    }
}

pub(super) async fn cleanup_staging_and_workspace(
    store: &dyn ArtifactStore,
    staging_key: Option<&str>,
    workspace: &dyn TempWorkspacePort,
    workspace_key: &WorkspaceKey,
) -> CleanupReport {
    let mut report = CleanupReport::new();
    if let Some(key) = staging_key {
        cleanup_staging(store, key, &mut report).await;
    }
    cleanup_workspace(workspace, workspace_key, &mut report).await;
    report
}
