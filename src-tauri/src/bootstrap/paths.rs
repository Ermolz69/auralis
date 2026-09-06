use std::path::{Path, PathBuf};
use tauri::Manager;

const DATABASE_FILE_NAME: &str = "auralis.sqlite";
const PROJECTS_DIRECTORY_NAME: &str = "projects";
const LOGS_DIRECTORY_NAME: &str = "logs";
const CACHE_DIRECTORY_NAME: &str = "cache";
const WORKSPACES_DIRECTORY_NAME: &str = "workspaces";
const NATIVE_E2E_DATA_DIR_ENV: &str = "AURALIS_NATIVE_E2E_DATA_DIR";

#[derive(Debug, thiserror::Error)]
pub enum AppPathsError {
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("{NATIVE_E2E_DATA_DIR_ENV} is required for a native E2E build")]
    MissingNativeE2eRoot,
    #[error("{NATIVE_E2E_DATA_DIR_ENV} must be an absolute path: {0}")]
    RelativeNativeE2eRoot(PathBuf),
}

/// Centralized layout for runtime files owned by the application.
///
/// The root is resolved by Tauri from the application bundle identifier, so
/// every supported OS keeps Auralis data outside the installation directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    pub fn resolve<R: tauri::Runtime, M: Manager<R>>(manager: &M) -> Result<Self, AppPathsError> {
        if native_e2e_build() {
            return native_e2e_root(std::env::var_os(NATIVE_E2E_DATA_DIR_ENV)).map(Self::new);
        }

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

fn native_e2e_build() -> bool {
    option_env!("AURALIS_NATIVE_E2E") == Some("1")
}

fn native_e2e_root(value: Option<std::ffi::OsString>) -> Result<PathBuf, AppPathsError> {
    let root = value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(AppPathsError::MissingNativeE2eRoot)?;
    if !root.is_absolute() {
        return Err(AppPathsError::RelativeNativeE2eRoot(root));
    }
    Ok(root)
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

    #[test]
    fn native_e2e_root_must_be_present_and_absolute() {
        assert!(matches!(
            native_e2e_root(None),
            Err(AppPathsError::MissingNativeE2eRoot)
        ));
        assert!(matches!(
            native_e2e_root(Some("relative/data".into())),
            Err(AppPathsError::RelativeNativeE2eRoot(_))
        ));

        let absolute = std::env::temp_dir().join("auralis-native-e2e");
        assert_eq!(
            native_e2e_root(Some(absolute.clone().into_os_string())).expect("absolute E2E root"),
            absolute
        );
    }
}
