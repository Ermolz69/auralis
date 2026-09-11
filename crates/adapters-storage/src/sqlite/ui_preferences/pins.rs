use super::repository::{conflict, error};
use domain::project::ProjectId;
use ports::{
    error::PortError,
    ui_preferences::{ProjectPin, ProjectPins},
};
use sqlx::SqlitePool;

pub(super) async fn get(pool: &SqlitePool) -> Result<ProjectPins, PortError> {
    let mut tx = pool.begin().await.map_err(error)?;
    let rows: Vec<(String, bool, i64)> =
        sqlx::query_as("SELECT project_id, pinned, revision FROM project_pins ORDER BY project_id")
            .fetch_all(&mut *tx)
            .await
            .map_err(error)?;
    let migrated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM ui_migrations WHERE key = 'project_pins_v1')",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(error)?;
    let entries = rows
        .into_iter()
        .map(|(id, pinned, revision)| {
            let project_id = id.parse().map_err(|_| PortError::InvalidStoredData {
                entity_type: "ProjectPin".into(),
                entity_id: id,
                field: "project_id".into(),
                message: "Invalid project identifier".into(),
            })?;
            Ok(ProjectPin {
                project_id,
                pinned,
                revision: revision as u64,
            })
        })
        .collect::<Result<_, PortError>>()?;
    tx.commit().await.map_err(error)?;
    Ok(ProjectPins { entries, migrated })
}
pub(super) async fn set(
    pool: &SqlitePool,
    id: &ProjectId,
    pinned: bool,
    expected: u64,
) -> Result<ProjectPin, PortError> {
    let revision: Option<i64> = if expected == 0 {
        sqlx::query_scalar("INSERT INTO project_pins(project_id, pinned, revision) SELECT id, ?, 1 FROM projects WHERE id = ? ON CONFLICT DO NOTHING RETURNING revision")
            .bind(pinned).bind(id.to_string()).fetch_optional(pool).await.map_err(error)?
    } else {
        sqlx::query_scalar("UPDATE project_pins SET pinned = ?, revision = revision + 1 WHERE project_id = ? AND revision = ? RETURNING revision")
            .bind(pinned).bind(id.to_string()).bind(i64::try_from(expected).unwrap_or(-1)).fetch_optional(pool).await.map_err(error)?
    };
    Ok(ProjectPin {
        project_id: id.clone(),
        pinned,
        revision: revision.ok_or_else(conflict)? as u64,
    })
}
pub(super) async fn import(
    pool: &SqlitePool,
    entries: &[(ProjectId, bool)],
) -> Result<ProjectPins, PortError> {
    let mut tx = pool.begin().await.map_err(error)?;
    let migrated: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM ui_migrations WHERE key = 'project_pins_v1')",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(error)?;
    if !migrated {
        for (id, pinned) in entries {
            sqlx::query("INSERT INTO project_pins(project_id, pinned, revision) SELECT id, ?, 1 FROM projects WHERE id = ? ON CONFLICT DO NOTHING")
                .bind(pinned).bind(id.to_string()).execute(&mut *tx).await.map_err(error)?;
        }
        sqlx::query("INSERT INTO ui_migrations(key) VALUES ('project_pins_v1')")
            .execute(&mut *tx)
            .await
            .map_err(error)?;
    }
    tx.commit().await.map_err(error)?;
    get(pool).await
}
