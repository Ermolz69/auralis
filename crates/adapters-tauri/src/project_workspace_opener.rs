use async_trait::async_trait;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::project_workspace::ProjectWorkspacePort;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

#[derive(Clone)]
pub struct ProjectWorkspaceOpener {
    app: AppHandle,
    projects_root: PathBuf,
}

impl ProjectWorkspaceOpener {
    pub fn new(app: AppHandle, projects_root: PathBuf) -> Self {
        Self { app, projects_root }
    }

    fn project_path(&self, project_id: &ProjectId) -> PathBuf {
        project_path(&self.projects_root, project_id)
    }
}

#[async_trait]
impl ProjectWorkspacePort for ProjectWorkspaceOpener {
    async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError> {
        let path = self.project_path(project_id);
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|_| PortError::Io {
                message: "Failed to create the project workspace".to_string(),
            })?;

        #[allow(deprecated)]
        self.app
            .shell()
            .open(path.to_string_lossy().into_owned(), None)
            .map_err(|_| PortError::Io {
                message: "Failed to open the project workspace".to_string(),
            })
    }
}

fn project_path(projects_root: &Path, project_id: &ProjectId) -> PathBuf {
    projects_root.join(project_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_path_is_derived_only_from_the_projects_root_and_typed_id() {
        let root = PathBuf::from("application-data").join("projects");
        let project_id = ProjectId::new();

        assert_eq!(
            project_path(&root, &project_id),
            root.join(project_id.to_string())
        );
    }
}
