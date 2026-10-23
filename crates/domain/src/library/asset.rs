use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LibraryAssetKind {
    Text,
    Subtitle,
    Audio,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryAsset {
    pub id: String,
    pub owner_project_id: String,
    pub name: String,
    pub kind: LibraryAssetKind,
    pub artifact_id: Option<String>,
    pub format: Option<String>,
    pub text: Option<String>,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryFolder {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryLink {
    pub asset_id: String,
    pub project_id: String,
    pub folder_id: Option<String>,
}
