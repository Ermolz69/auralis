use std::path::{Path, PathBuf};
use tauri::Manager;

const DATABASE_FILE_NAME: &str = "auralis.sqlite";
const PROJECTS_DIRECTORY_NAME: &str = "projects";
const LOGS_DIRECTORY_NAME: &str = "logs";
const CACHE_DIRECTORY_NAME: &str = "cache";
const WORKSPACES_DIRECTORY_NAME: &str = "workspaces";

/// Centralized layout for runtime files owned by the application.
///
/// The root is resolved by Tauri from the application bundle identifier, so
/// every supported OS keeps Auralis data outside the installation directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn resolve<R: tauri::Runtime, M: Manager<R>>(manager: &M) -> Result<Self, tauri::Error> {
        Ok(Self::new(manager.path().app_data_dir()?))
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        self.root.join(DATABASE_FILE_NAME)
    }

    pub fn projects(&self) -> PathBuf {
        self.root.join(PROJECTS_DIRECTORY_NAME)
    }

    pub fn project(&self, project_id: &domain::project::ProjectId) -> PathBuf {
        self.projects().join(project_id.to_string())
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join(LOGS_DIRECTORY_NAME)
    }

    pub fn workspaces(&self) -> PathBuf {
        self.root
            .join(CACHE_DIRECTORY_NAME)
            .join(WORKSPACES_DIRECTORY_NAME)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn derives_database_under_app_root() {
        let root = PathBuf::from("application-data");
        let paths = AppPaths::new(root.clone());

        assert_eq!(paths.database(), root.join("auralis.sqlite"));
    }

    #[test]
    fn derives_projects_under_app_root() {
        let root = PathBuf::from("application-data");
        let paths = AppPaths::new(root.clone());

        assert_eq!(paths.projects(), root.join("projects"));
    }

    #[test]
    fn derives_typed_project_directory() {
        let root = PathBuf::from("application-data");
        let paths = AppPaths::new(root.clone());
        let project_id = domain::project::ProjectId::new();

        assert_eq!(
            paths.project(&project_id),
            root.join("projects").join(project_id.to_string())
        );
    }

    #[test]
    fn derives_logs_and_workspaces() {
        let root = PathBuf::from("application-data");
        let paths = AppPaths::new(root.clone());

        assert_eq!(paths.logs(), root.join("logs"));
        assert_eq!(paths.workspaces(), root.join("cache").join("workspaces"));
    }

    #[test]
    fn isolates_project_directories() {
        let paths = AppPaths::new(PathBuf::from("application-data"));
        let first = "00000000-0000-0000-0000-000000000001"
            .parse::<domain::project::ProjectId>()
            .expect("valid first project ID");
        let second = "00000000-0000-0000-0000-000000000002"
            .parse::<domain::project::ProjectId>()
            .expect("valid second project ID");

        assert_ne!(paths.project(&first), paths.project(&second));
    }
}
