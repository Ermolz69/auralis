use std::sync::Arc;

use crate::error::{ApplicationError, CleanupReport};
use crate::usecases::import_cleanup::{cleanup_staging_and_workspace, cleanup_workspace};
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

    pub async fn cleanup_workspace(&self, key: &WorkspaceKey) -> CleanupReport {
        let mut report = CleanupReport::new();
        cleanup_workspace(self.workspace_port.as_ref(), key, &mut report).await;
        report
    }

    pub async fn cleanup_all(
        &self,
        staging_key: &str,
        workspace_key: &WorkspaceKey,
    ) -> CleanupReport {
        cleanup_staging_and_workspace(
            self.artifact_store.as_ref(),
            Some(staging_key),
            self.workspace_port.as_ref(),
            workspace_key,
        )
        .await
    }

    pub async fn handle_workspace_failure(
        &self,
        alloc_key: &WorkspaceKey,
        primary_err: ApplicationError,
    ) -> ApplicationError {
        self.cleanup_workspace(alloc_key)
            .await
            .into_error(primary_err)
    }

    pub async fn handle_all_failure(
        &self,
        staging_key: &str,
        alloc_key: &WorkspaceKey,
        primary_err: ApplicationError,
    ) -> ApplicationError {
        self.cleanup_all(staging_key, alloc_key)
            .await
            .into_error(primary_err)
    }
}
