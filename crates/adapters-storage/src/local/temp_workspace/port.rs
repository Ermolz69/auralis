use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use uuid::Uuid;

use domain::outbox::WorkspaceKey;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::workspace::{TempWorkspacePort, WorkspaceAllocation, WorkspaceCleanupReport};

use super::janitor::TempWorkspaceJanitor;

pub struct LocalTempWorkspace {
    workspace_root: PathBuf,
}

impl LocalTempWorkspace {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }
}

#[async_trait]
impl TempWorkspacePort for LocalTempWorkspace {
    async fn create_allocation(
        &self,
        project_id: &ProjectId,
        purpose: &str,
    ) -> Result<WorkspaceAllocation, PortError> {
        let allocation_id = Uuid::new_v4().to_string();
        // Structure: tmp/{project_id}/{purpose}_{allocation_id}
        let relative_path = format!("tmp/{}/{}_{}", project_id, purpose, allocation_id);

        let workspace_key = WorkspaceKey::new(relative_path).map_err(|e| PortError::Io {
            message: format!("Invalid input: {}", e),
        })?;

        let absolute_path = self.workspace_root.join(workspace_key.as_str());

        if let Err(e) = tokio::fs::create_dir_all(&absolute_path).await {
            return Err(PortError::Io {
                message: e.to_string(),
            });
        }

        Ok(WorkspaceAllocation {
            absolute_path,
            workspace_key,
            allocation_id,
        })
    }

    #[allow(clippy::collapsible_if)]
    async fn delete_allocation(&self, key: &WorkspaceKey) -> Result<(), PortError> {
        let path = self.resolve_key(key).await?;

        // Ensure path doesn't escape workspace root via symlink
        if let Ok(metadata) = tokio::fs::symlink_metadata(&path).await {
            if metadata.is_symlink() {
                return Err(PortError::Io {
                    message: "Path escapes workspace root".to_string(),
                });
            }
        }

        if path.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&path).await {
                return Err(PortError::Io {
                    message: e.to_string(),
                });
            }
        }
        Ok(())
    }

    async fn resolve_key(&self, key: &WorkspaceKey) -> Result<PathBuf, PortError> {
        // Resolve exactly against workspace_root
        // WorkspaceKey already prevents `..` and absolute paths lexically.
        let target_path = self.workspace_root.join(key.as_str());
        Ok(target_path)
    }

    async fn cleanup_stale_allocations(
        &self,
        age_threshold: Duration,
    ) -> Result<WorkspaceCleanupReport, PortError> {
        let janitor = TempWorkspaceJanitor::new(self.workspace_root.clone(), age_threshold);
        janitor.run().await
    }
}
