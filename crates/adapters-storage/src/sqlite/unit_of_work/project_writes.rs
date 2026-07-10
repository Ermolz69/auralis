use domain::project::Project;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::project_mapper::project_to_row_values;

pub(super) async fn save_project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), PortError> {
    let row = project_to_row_values(project)?;
    sqlx::query(
        r#"
        INSERT INTO projects (
            id, title, status, source_json, metadata_json, 
            source_language, target_language, transcript_json, 
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            status = excluded.status,
            source_json = excluded.source_json,
            metadata_json = excluded.metadata_json,
            source_language = excluded.source_language,
            target_language = excluded.target_language,
            transcript_json = excluded.transcript_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(row.id)
    .bind(row.title)
    .bind(row.status)
    .bind(row.source_json)
    .bind(row.metadata_json)
    .bind(row.source_language)
    .bind(row.target_language)
    .bind(row.transcript_json)
    .bind(row.created_at)
    .bind(row.updated_at)
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to save project in tx: {}", e),
    })?;
    Ok(())
}
