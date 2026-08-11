use std::path::{Path, PathBuf};

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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn keeps_all_runtime_paths_under_the_application_root() {
        let root = PathBuf::from("application-data");
        let paths = AppPaths::new(root.clone());

        assert_eq!(paths.root(), root);
        assert_eq!(paths.database(), root.join("auralis.sqlite"));
        assert_eq!(paths.projects(), root.join("projects"));
        assert_eq!(paths.logs(), root.join("logs"));
        assert_eq!(paths.workspaces(), root.join("cache").join("workspaces"));
    }
}
