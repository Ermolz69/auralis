use domain::project::Project;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::project_mapper::project_to_row_values;

pub(super) async fn update_project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), PortError> {
    let row = project_to_row_values(project)?;
    let result = sqlx::query(
        r#"
        UPDATE projects SET
            title = ?,
            status = ?,
            source_json = ?,
            metadata_json = ?,
            source_language = ?,
            target_language = ?,
            transcript_json = ?,
            active_job_id = ?,
            last_terminal_job_id = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(row.title)
    .bind(row.status)
    .bind(row.source_json)
    .bind(row.metadata_json)
    .bind(row.source_language)
    .bind(row.target_language)
    .bind(row.transcript_json)
    .bind(row.active_job_id)
    .bind(row.last_terminal_job_id)
    .bind(row.updated_at)
    .bind(row.id)
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to update project in tx: {}", e),
    })?;

    if result.rows_affected() == 0 {
        return Err(PortError::NotFound {
            resource: "Project".to_string(),
        });
    }

    Ok(())
}
