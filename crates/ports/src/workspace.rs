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

    /// Reads the content of a file within the workspace specified by WorkspaceKey as a string.
    /// The filename is a relative path within that allocation, containing only Normal components.
    /// The read is bounded to max_bytes, and fails if it exceeds or has invalid UTF-8 encoding.
    async fn read_workspace_file_to_string(
        &self,
        allocation_key: &WorkspaceKey,
        relative_file: &str,
        max_bytes: u64,
    ) -> Result<String, PortError>;

    /// Safely resolves a child path within the workspace allocation to an absolute PathBuf,
    /// ensuring it is contained and not a symlink/traversal.
    async fn resolve_child_path(
        &self,
        key: &WorkspaceKey,
        relative_file: &str,
    ) -> Result<PathBuf, PortError>;
}
