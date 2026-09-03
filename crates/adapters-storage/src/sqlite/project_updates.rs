use domain::project::{Project, ProjectId};
use ports::{error::PortError, project_update::ProjectUpdate};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use super::{
    helpers::{map_sqlite_error, serialize_json},
    project_mapper::row_to_project,
    project_row::ProjectRow,
};

pub(super) async fn update_project(
    pool: &SqlitePool,
    id: &ProjectId,
    expected_revision: u64,
    update: ProjectUpdate,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> Result<Project, PortError> {
    let mut query =
        QueryBuilder::<Sqlite>::new("UPDATE projects SET revision = revision + 1, updated_at = ");
    query.push_bind(updated_at.to_rfc3339());
    let expected_status = match update {
        ProjectUpdate::Rename { title } => {
            if title.trim().is_empty() {
                return Err(PortError::Conflict {
                    resource: "Project".to_string(),
                    message: "Project title cannot be empty".to_string(),
                });
            }
            query.push(", title = ").push_bind(title.trim().to_string());
            None
        }
        ProjectUpdate::ImportSource { source, metadata } => {
            query
                .push(", status = 'SourceImported', source_json = ")
                .push_bind(serialize_json(&source, "source")?)
                .push(", metadata_json = ")
                .push_bind(
                    metadata
                        .map(|value| serialize_json(&value, "metadata"))
                        .transpose()?,
                );
            Some("Draft")
        }
        ProjectUpdate::MarkReadyForProcessing => {
            query.push(", status = 'ReadyForProcessing'");
            Some("SourceImported")
        }
    };
    query
        .push(" WHERE id = ")
        .push_bind(id.to_string())
        .push(" AND revision = ")
        .push_bind(i64::try_from(expected_revision).unwrap_or(-1))
        .push(" AND revision < 9223372036854775807");
    if let Some(status) = expected_status {
        query.push(" AND status = ").push_bind(status);
    }
    query.push(" RETURNING *");
    let row = query
        .build_query_as::<ProjectRow>()
        .fetch_optional(pool)
        .await
        .map_err(|e| map_sqlite_error("update_project", e))?;
    match row {
        Some(row) => row_to_project(row),
        None => Err(project_write_error(pool, id).await?),
    }
}

pub(super) async fn project_write_error<'e, E>(
    executor: E,
    id: &ProjectId,
) -> Result<PortError, PortError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?)")
        .bind(id.to_string())
        .fetch_one(executor)
        .await
        .map_err(|e| map_sqlite_error("check_project_existence", e))?;
    Ok(if exists {
        PortError::Conflict {
            resource: "Project".to_string(),
            message: "Project revision or state changed concurrently".to_string(),
        }
    } else {
        PortError::NotFound {
            resource: "Project".to_string(),
        }
    })
}
