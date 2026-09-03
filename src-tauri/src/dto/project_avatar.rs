#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAvatarDto {
    pub data_url: Option<String>,
    pub initialized: bool,
}

impl From<ports::project_avatar::ProjectAvatarRecord> for ProjectAvatarDto {
    fn from(record: ports::project_avatar::ProjectAvatarRecord) -> Self {
        Self {
            data_url: record.avatar.map(|avatar| avatar.as_str().to_string()),
            initialized: record.initialized,
        }
    }
}
