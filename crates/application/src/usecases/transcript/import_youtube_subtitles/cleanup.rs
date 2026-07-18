use std::sync::Arc;

use domain::outbox::WorkspaceKey;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

pub struct ImportCleanupCoordinator {
    artifact_store: Arc<dyn ArtifactStore>,
    workspace_port: Arc<dyn TempWorkspacePort>,
}

impl ImportCleanupCoordinator {
    pub fn new(
        artifact_store: Arc<dyn ArtifactStore>,
        workspace_port: Arc<dyn TempWorkspacePort>,
    ) -> Self {
        Self {
            artifact_store,
            workspace_port,
        }
    }

    /// Cleans up the workspace allocation.
    /// If an error occurs, logs it. We do NOT return the error here because
    /// this is typically called during an existing failure path.
    pub async fn cleanup_workspace(&self, key: &WorkspaceKey) -> crate::error::CleanupReport {
        let mut report = crate::error::CleanupReport::new();
        if let Err(e) = self.workspace_port.delete_allocation(key).await {
            report.add_failure(crate::error::CleanupTarget::workspace(key.to_string()), e);
        }
        report
    }

    /// Cleans up both artifact staging and workspace allocation.
    pub async fn cleanup_all(
        &self,
        staging_key: &str,
        workspace_key: &WorkspaceKey,
    ) -> crate::error::CleanupReport {
        let mut report = crate::error::CleanupReport::new();

        if let Err(e) = self.artifact_store.delete_storage_key(staging_key).await {
            report.add_failure(crate::error::CleanupTarget::staging(staging_key), e);
        }

        if let Err(e) = self.workspace_port.delete_allocation(workspace_key).await {
            report.add_failure(
                crate::error::CleanupTarget::workspace(workspace_key.to_string()),
                e,
            );
        }

        report
    }
}
