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
    pub async fn cleanup_workspace(&self, key: &WorkspaceKey) {
        if let Err(e) = self.workspace_port.delete_allocation(key).await {
            eprintln!("Failed to clean up workspace allocation {}: {:?}", key, e);
        }
    }

    /// Cleans up both artifact staging and workspace allocation.
    pub async fn cleanup_all(&self, staging_key: &str, workspace_key: &WorkspaceKey) {
        if let Err(e) = self.artifact_store.delete_storage_key(staging_key).await {
            eprintln!("Failed to clean up staging key {}: {:?}", staging_key, e);
        }
        self.cleanup_workspace(workspace_key).await;
    }
}
