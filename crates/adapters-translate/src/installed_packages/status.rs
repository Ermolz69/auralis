use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationPackageStatus {
    pub schema_version: u32,
    pub release_id: String,
    pub backend: String,
    pub supported: bool,
    pub installed: bool,
    pub selected: bool,
}
