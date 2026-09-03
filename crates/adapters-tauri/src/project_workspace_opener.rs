use async_trait::async_trait;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::project_workspace::ProjectWorkspacePort;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

#[derive(Clone)]
pub struct ProjectWorkspaceOpener {
    operations: Arc<dyn WorkspaceOperations>,
    projects_root: PathBuf,
}

impl ProjectWorkspaceOpener {
    pub fn new(app: AppHandle, projects_root: PathBuf) -> Self {
        Self {
            operations: Arc::new(SystemWorkspaceOperations { app }),
            projects_root,
        }
    }

    fn project_path(&self, project_id: &ProjectId) -> PathBuf {
        project_path(&self.projects_root, project_id)
    }

    #[cfg(test)]
    fn with_operations(projects_root: PathBuf, operations: Arc<dyn WorkspaceOperations>) -> Self {
        Self {
            operations,
            projects_root,
        }
    }
}

#[async_trait]
impl ProjectWorkspacePort for ProjectWorkspaceOpener {
    async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError> {
        let path = self.project_path(project_id);
        self.operations.ensure_directory(&path).await?;
        self.operations.open_path(&path)
    }
}

#[async_trait]
trait WorkspaceOperations: Send + Sync {
    async fn ensure_directory(&self, path: &Path) -> Result<(), PortError>;
    fn open_path(&self, path: &Path) -> Result<(), PortError>;
}

struct SystemWorkspaceOperations {
    app: AppHandle,
}

#[async_trait]
impl WorkspaceOperations for SystemWorkspaceOperations {
    async fn ensure_directory(&self, path: &Path) -> Result<(), PortError> {
        ensure_directory(path).await
    }

    fn open_path(&self, path: &Path) -> Result<(), PortError> {
        open_path(&self.app, path)
    }
}

async fn ensure_directory(path: &Path) -> Result<(), PortError> {
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|_| PortError::Io {
            message: "Failed to create the project workspace".to_string(),
        })
}

fn open_path(app: &AppHandle, path: &Path) -> Result<(), PortError> {
    #[allow(deprecated)]
    app.shell()
        .open(path.to_string_lossy().into_owned(), None)
        .map_err(|_| PortError::Io {
            message: "Failed to open the project workspace".to_string(),
        })
}

fn project_path(projects_root: &Path, project_id: &ProjectId) -> PathBuf {
    projects_root.join(project_id.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct TestWorkspaceOperations {
        opened: Mutex<Vec<PathBuf>>,
        fail_open: bool,
    }

    #[async_trait]
    impl WorkspaceOperations for TestWorkspaceOperations {
        async fn ensure_directory(&self, path: &Path) -> Result<(), PortError> {
            ensure_directory(path).await
        }

        fn open_path(&self, path: &Path) -> Result<(), PortError> {
            self.opened.lock().unwrap().push(path.to_path_buf());
            if self.fail_open {
                return Err(PortError::Io {
                    message: "Failed to open the project workspace".to_string(),
                });
            }
            Ok(())
        }
    }

    #[test]
    fn project_path_is_derived_only_from_the_projects_root_and_typed_id() {
        let root = PathBuf::from("application-data").join("projects");
        let project_id = ProjectId::new();

        assert_eq!(
            project_path(&root, &project_id),
            root.join(project_id.to_string())
        );
    }

    #[tokio::test]
    async fn existing_directory_can_be_opened_repeatedly() {
        let temp = tempfile::tempdir().unwrap();
        let project_id = ProjectId::new();
        let operations = Arc::new(TestWorkspaceOperations::default());
        let opener = ProjectWorkspaceOpener::with_operations(
            temp.path().join("projects"),
            operations.clone(),
        );

        opener.ensure_and_open(&project_id).await.unwrap();
        opener.ensure_and_open(&project_id).await.unwrap();

        let expected_path = temp.path().join("projects").join(project_id.to_string());
        assert!(expected_path.is_dir());
        assert_eq!(
            *operations.opened.lock().unwrap(),
            vec![expected_path.clone(), expected_path]
        );
    }

    #[tokio::test]
    async fn create_directory_failure_does_not_invoke_shell() {
        let temp = tempfile::tempdir().unwrap();
        let project_id = ProjectId::new();
        let projects_root = temp.path().join("projects");
        std::fs::create_dir_all(&projects_root).unwrap();
        std::fs::write(
            projects_root.join(project_id.to_string()),
            "not a directory",
        )
        .unwrap();
        let operations = Arc::new(TestWorkspaceOperations::default());
        let opener = ProjectWorkspaceOpener::with_operations(projects_root, operations.clone());

        let result = opener.ensure_and_open(&project_id).await;

        assert!(matches!(result, Err(PortError::Io { .. })));
        assert!(operations.opened.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn shell_failure_is_returned_as_io_error() {
        let temp = tempfile::tempdir().unwrap();
        let project_id = ProjectId::new();
        let operations = Arc::new(TestWorkspaceOperations {
            opened: Mutex::new(Vec::new()),
            fail_open: true,
        });
        let opener = ProjectWorkspaceOpener::with_operations(
            temp.path().join("projects"),
            operations.clone(),
        );

        let result = opener.ensure_and_open(&project_id).await;

        assert!(matches!(
            result,
            Err(PortError::Io { message }) if message == "Failed to open the project workspace"
        ));
        assert_eq!(operations.opened.lock().unwrap().len(), 1);
    }
}
