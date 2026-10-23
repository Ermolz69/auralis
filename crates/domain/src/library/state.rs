use super::{LibraryAsset, LibraryFolder, LibraryLink, LibraryWorkspace};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryBrowser {
    pub scope: String,
    pub project_id: Option<String>,
    pub folder_id: Option<String>,
    pub search: String,
    pub kind: String,
    pub selected_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryState {
    pub revision: u64,
    pub assets: Vec<LibraryAsset>,
    pub folders: Vec<LibraryFolder>,
    pub links: Vec<LibraryLink>,
    pub standalone_owners: Vec<String>,
    pub removed_projects: Vec<String>,
    pub hidden_assets: Vec<String>,
    pub workspaces: Vec<LibraryWorkspace>,
    pub browser: LibraryBrowser,
}
