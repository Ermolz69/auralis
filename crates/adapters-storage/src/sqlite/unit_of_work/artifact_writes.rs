use domain::media::Artifact;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::artifact_index::mapper::artifact_to_row_values;

pub(super) async fn save_artifact(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &domain::project::ProjectId,
    artifact: &Artifact,
) -> Result<(), PortError> {
    let values = artifact_to_row_values(project_id, artifact)?;
    sqlx::query(
        r#"
        INSERT INTO artifacts (
            id, project_id, kind, location_kind, location_value, size_bytes, 
            state, created_at, updated_at, ready_at
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
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to add artifact in tx", e))?;
    Ok(())
}
