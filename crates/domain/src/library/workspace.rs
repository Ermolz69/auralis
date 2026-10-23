use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryTool {
    Source,
    Text,
    Translate,
    Speech,
    Voices,
    Results,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryTab {
    pub id: String,
    pub tool: LibraryTool,
    pub asset_id: Option<String>,
    #[serde(default)]
    pub draft: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryWorkspace {
    pub id: String,
    pub project_id: Option<String>,
    pub asset_id: Option<String>,
    pub active_tab: Option<String>,
    pub tabs: Vec<LibraryTab>,
}
