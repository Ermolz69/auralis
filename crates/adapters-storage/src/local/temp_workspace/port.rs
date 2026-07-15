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

        let absolute_path = self.resolve_key(&workspace_key).await?;

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

        if path.exists() {
            if let Err(e) = tokio::fs::remove_dir_all(&path).await {
                // If it fails because it's not a directory (legacy file path), we fallback to remove_file
                if e.kind() == std::io::ErrorKind::NotADirectory {
                    if let Err(e2) = tokio::fs::remove_file(&path).await {
                        return Err(PortError::Io {
                            message: format!("Failed to remove file: {}", e2),
                        });
                    }
                    return Ok(());
                }
                return Err(PortError::Io {
                    message: format!("Failed to remove directory: {}", e),
                });
            }
        }
        Ok(())
    }

    async fn resolve_key(&self, key: &WorkspaceKey) -> Result<PathBuf, PortError> {
        let key_str = key.as_str();
        if !key_str.starts_with("tmp/") && !key_str.starts_with("tmp\\") {
            return Err(PortError::Io {
                message: "WorkspaceKey must start with 'tmp/'".to_string(),
            });
        }

        let canonical_root = tokio::fs::canonicalize(&self.workspace_root)
            .await
            .unwrap_or_else(|_| self.workspace_root.clone());

        let target_path = canonical_root.join(key_str);

        // Find closest existing ancestor
        let mut ancestor = target_path.clone();
        while !ancestor.exists() {
            if let Some(parent) = ancestor.parent() {
                ancestor = parent.to_path_buf();
            } else {
                break;
            }
        }

        if ancestor.exists() {
            let canonical_ancestor =
                tokio::fs::canonicalize(&ancestor)
                    .await
                    .map_err(|e| PortError::Io {
                        message: format!("Failed to canonicalize ancestor: {}", e),
                    })?;

            if !canonical_ancestor.starts_with(&canonical_root) {
                return Err(PortError::Io {
                    message: "Path escapes workspace root".to_string(),
                });
            }

            // Reject any symlink in the ancestor
            let metadata = tokio::fs::symlink_metadata(&canonical_ancestor)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to stat ancestor: {}", e),
                })?;

            if metadata.is_symlink() {
                return Err(PortError::Io {
                    message: "Symlink components are not allowed".to_string(),
                });
            }
        }

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
