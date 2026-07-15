use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

use domain::outbox::WorkspaceKey;
use domain::project::ProjectId;

use crate::error::PortError;

pub struct WorkspaceAllocation {
    pub absolute_path: PathBuf,
    pub workspace_key: WorkspaceKey,
    pub allocation_id: String,
}

#[derive(Debug)]
pub struct WorkspaceCleanupReport {
    pub deleted_count: usize,
    pub failed_count: usize,
}

#[async_trait]
pub trait TempWorkspacePort: Send + Sync {
    /// Creates a unique workspace allocation (directory) for a project/purpose.
    async fn create_allocation(
        &self,
        project_id: &ProjectId,
        purpose: &str,
    ) -> Result<WorkspaceAllocation, PortError>;

    /// Deletes an allocation and all its contents by its WorkspaceKey.
    async fn delete_allocation(&self, key: &WorkspaceKey) -> Result<(), PortError>;

    /// Resolves a WorkspaceKey to an absolute PathBuf, ensuring no symlink escapes.
    async fn resolve_key(&self, key: &WorkspaceKey) -> Result<PathBuf, PortError>;

    /// Cleans up stale allocations older than the given threshold.
    async fn cleanup_stale_allocations(
        &self,
        age_threshold: Duration,
    ) -> Result<WorkspaceCleanupReport, PortError>;
}
