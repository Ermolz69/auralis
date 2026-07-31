use sqlx::SqlitePool;

use domain::media::{Artifact, ArtifactId};
use domain::project::ProjectId;
use ports::error::PortError;

use super::mapper::artifact_to_row_values;
use super::serialization::artifact_state_to_db;

pub async fn upsert_artifact(
    pool: &SqlitePool,
    project_id: &ProjectId,
    artifact: &Artifact,
) -> Result<(), PortError> {
    let values = artifact_to_row_values(project_id, artifact)?;

    sqlx::query(
        r#"
        INSERT INTO artifacts (
            id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            kind = excluded.kind,
            location_kind = excluded.location_kind,
            location_value = excluded.location_value,
            size_bytes = excluded.size_bytes,
            state = excluded.state,
            updated_at = excluded.updated_at,
            ready_at = excluded.ready_at
        "#,
    )
    .bind(values.id)
    .bind(values.project_id)
    .bind(values.kind)
    .bind(values.location_kind)
    .bind(values.location_value)
    .bind(values.size_bytes)
    .bind(values.state)
    .bind(values.created_at)
    .bind(values.updated_at)
    .bind(values.ready_at)
    .execute(pool)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("upsert_artifact", e))?;

    Ok(())
}

pub async fn delete_artifact(pool: &SqlitePool, id: &ArtifactId) -> Result<(), PortError> {
    sqlx::query("DELETE FROM artifacts WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("delete_artifact", e))?;
    Ok(())
}

pub async fn update_artifact_state(
    pool: &SqlitePool,
    id: &ArtifactId,
    state: domain::media::ArtifactState,
    ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
) -> Result<(), PortError> {
    let state_str = artifact_state_to_db(&state)?;
    let ready_at_str = ready_at.map(|dt| dt.to_rfc3339());
    let updated_at_str = domain::chrono::Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE artifacts
        SET state = ?, updated_at = ?, ready_at = coalesce(?, ready_at)
        WHERE id = ?
        "#,
    )
    .bind(state_str)
    .bind(updated_at_str)
    .bind(ready_at_str)
    .bind(id.to_string())
    .execute(pool)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("update_artifact_state", e))?;

    Ok(())
}
