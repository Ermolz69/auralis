use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncReadExt;

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

            if canonical_ancestor.strip_prefix(&canonical_root).is_err() {
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

    async fn read_workspace_file_to_string(
        &self,
        allocation_key: &WorkspaceKey,
        relative_file: &str,
        max_bytes: u64,
    ) -> Result<String, PortError> {
        let file_path = self
            .resolve_child_path(allocation_key, relative_file)
            .await?;

        let metadata =
            tokio::fs::symlink_metadata(&file_path)
                .await
                .map_err(|_| PortError::Io {
                    message: "File not found".to_string(),
                })?;

        if !metadata.is_file() {
            return Err(PortError::Io {
                message: "Not a regular file".to_string(),
            });
        }

        // Bounded overflow check
        if metadata.len() > max_bytes {
            return Err(PortError::Io {
                message: "File size exceeds limit".to_string(),
            });
        }

        let file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|_| PortError::Io {
                message: "Failed to open file".to_string(),
            })?;

        // Bounded read up to max_bytes + 1
        let read_limit = max_bytes.checked_add(1).unwrap_or(max_bytes);
        let mut buffer = Vec::with_capacity(std::cmp::min(metadata.len() as usize, 4096));
        let mut handle = file.take(read_limit);
        handle
            .read_to_end(&mut buffer)
            .await
            .map_err(|_| PortError::Io {
                message: "Failed to read file".to_string(),
            })?;

        if buffer.len() > max_bytes as usize {
            return Err(PortError::Io {
                message: "File size exceeds limit".to_string(),
            });
        }

        let text = String::from_utf8(buffer).map_err(|_| PortError::Io {
            message: "Invalid UTF-8 encoding".to_string(),
        })?;

        Ok(text)
    }

    async fn resolve_child_path(
        &self,
        key: &WorkspaceKey,
        relative_file: &str,
    ) -> Result<PathBuf, PortError> {
        let allocation_dir = self.resolve_key(key).await?;

        if relative_file.is_empty() {
            return Err(PortError::Io {
                message: "Empty child path".to_string(),
            });
        }

        let rel_path = std::path::Path::new(relative_file);
        for component in rel_path.components() {
            match component {
                std::path::Component::Normal(_) => {}
                _ => {
                    return Err(PortError::Io {
                        message: "Invalid child path: absolute paths or traversal components are forbidden".to_string(),
                    });
                }
            }
        }

        let file_path = allocation_dir.join(relative_file);

        let canonical_allocation =
            tokio::fs::canonicalize(&allocation_dir)
                .await
                .map_err(|_| PortError::Io {
                    message: "Workspace directory not found".to_string(),
                })?;

        // If target file/dir exists, verify containment and symlink rules
        if file_path.exists() {
            let canonical_file =
                tokio::fs::canonicalize(&file_path)
                    .await
                    .map_err(|_| PortError::Io {
                        message: "Failed to canonicalize child path".to_string(),
                    })?;

            if !canonical_file.starts_with(&canonical_allocation) {
                return Err(PortError::Io {
                    message: "Path traversal attempt detected".to_string(),
                });
            }

            // Ensure no symlinks (both terminal target and intermediate components)
            let mut current = file_path.as_path();
            while let Some(parent) = current.parent() {
                if parent == allocation_dir || parent.as_os_str().is_empty() {
                    break;
                }
                let is_sym = tokio::fs::symlink_metadata(current)
                    .await
                    .map(|m| m.is_symlink())
                    .unwrap_or(false);
                if is_sym {
                    return Err(PortError::Io {
                        message: "Symlink component in path is forbidden".to_string(),
                    });
                }
                current = parent;
            }

            let is_target_sym = tokio::fs::symlink_metadata(&file_path)
                .await
                .map(|m| m.is_symlink())
                .unwrap_or(false);
            if is_target_sym {
                return Err(PortError::Io {
                    message: "Symlinks are forbidden".to_string(),
                });
            }
        }

        Ok(file_path)
    }
}
