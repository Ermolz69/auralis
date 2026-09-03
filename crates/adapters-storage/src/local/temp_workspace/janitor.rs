use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ports::error::PortError;
use ports::workspace::WorkspaceCleanupReport;

#[async_trait::async_trait]
pub(crate) trait JanitorOps: Send + Sync {
    async fn remove_dir_all(&self, path: &std::path::Path) -> std::io::Result<()>;
}

pub(crate) struct DefaultJanitorOps;

#[async_trait::async_trait]
impl JanitorOps for DefaultJanitorOps {
    async fn remove_dir_all(&self, path: &std::path::Path) -> std::io::Result<()> {
        tokio::fs::remove_dir_all(path).await
    }
}

pub struct TempWorkspaceJanitor {
    workspace_root: PathBuf,
    age_threshold: Duration,
    ops: Box<dyn JanitorOps>,
}

impl TempWorkspaceJanitor {
    pub fn new(workspace_root: PathBuf, age_threshold: Duration) -> Self {
        Self {
            workspace_root,
            age_threshold,
            ops: Box::new(DefaultJanitorOps),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_ops(
        workspace_root: PathBuf,
        age_threshold: Duration,
        ops: Box<dyn JanitorOps>,
    ) -> Self {
        Self {
            workspace_root,
            age_threshold,
            ops,
        }
    }

    #[allow(clippy::collapsible_if)]
    pub async fn run(&self) -> Result<WorkspaceCleanupReport, PortError> {
        let mut report = WorkspaceCleanupReport {
            deleted_count: 0,
            failed_count: 0,
        };

        let tmp_dir = self.workspace_root.join("tmp");
        if !tmp_dir.exists() {
            return Ok(report);
        }

        let mut project_dirs = match tokio::fs::read_dir(&tmp_dir).await {
            Ok(d) => d,
            Err(e) => {
                return Err(PortError::Io {
                    message: e.to_string(),
                });
            }
        };

        let cutoff_time = SystemTime::now() - self.age_threshold;

        while let Ok(Some(project_entry)) = project_dirs.next_entry().await {
            if let Ok(file_type) = project_entry.file_type().await {
                if file_type.is_symlink() {
                    continue; // Skip symlinks completely
                }
            }

            if let Ok(mut alloc_dirs) = tokio::fs::read_dir(project_entry.path()).await {
                while let Ok(Some(alloc_entry)) = alloc_dirs.next_entry().await {
                    if let Ok(metadata) = tokio::fs::symlink_metadata(alloc_entry.path()).await {
                        if metadata.is_symlink() {
                            continue;
                        }

                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff_time {
                                if self.ops.remove_dir_all(&alloc_entry.path()).await.is_ok() {
                                    report.deleted_count += 1;
                                } else {
                                    report.failed_count += 1;
                                }
                            }
                        }
                    }
                }
            }

            // Clean up empty project dirs
            let _ = tokio::fs::remove_dir(project_entry.path()).await;
        }

        Ok(report)
    }
}
