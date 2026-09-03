use super::helpers::map_sqlite_error;
use async_trait::async_trait;
use domain::project::{ProjectId, avatar::ProjectAvatar};
use ports::{
    error::PortError,
    project_avatar::{ProjectAvatarRecord, ProjectAvatarRepository},
};
use sqlx::SqlitePool;

pub struct SqliteProjectAvatarRepository {
    pool: SqlitePool,
}

impl SqliteProjectAvatarRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn decode(
    project_id: &ProjectId,
    row: Option<Option<String>>,
) -> Result<ProjectAvatarRecord, PortError> {
    let value = row.ok_or_else(|| PortError::NotFound {
        resource: format!("Project {project_id}"),
    })?;
    let initialized = value.is_some();
    let avatar = value
        .filter(|value| !value.is_empty())
        .map(ProjectAvatar::new)
        .transpose()
        .map_err(|_| PortError::InvalidStoredData {
            entity_type: "project".into(),
            entity_id: project_id.to_string(),
            field: "avatar_data_url".into(),
            message: "Invalid stored project avatar".into(),
        })?;
    Ok(ProjectAvatarRecord {
        avatar,
        initialized,
    })
}

#[async_trait]
impl ProjectAvatarRepository for SqliteProjectAvatarRepository {
    async fn get(&self, project_id: &ProjectId) -> Result<ProjectAvatarRecord, PortError> {
        let row = sqlx::query_scalar::<_, Option<String>>(
            "SELECT avatar_data_url FROM projects WHERE id = ?",
        )
        .bind(project_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_sqlite_error("get_project_avatar", e))?;
        decode(project_id, row)
    }
    async fn set(
        &self,
        project_id: &ProjectId,
        avatar: Option<ProjectAvatar>,
        only_if_missing: bool,
    ) -> Result<ProjectAvatarRecord, PortError> {
        let value = avatar.as_ref().map_or("", ProjectAvatar::as_str);
        let row = sqlx::query_scalar::<_, Option<String>>("UPDATE projects SET avatar_data_url = CASE WHEN ? THEN COALESCE(avatar_data_url, ?) ELSE ? END WHERE id = ? RETURNING avatar_data_url")
            .bind(only_if_missing).bind(value).bind(value).bind(project_id.to_string()).fetch_optional(&self.pool).await.map_err(|e| map_sqlite_error("set_project_avatar", e))?;
        decode(project_id, row)
    }
}
