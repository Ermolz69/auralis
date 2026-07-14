use domain::job::Job;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::job_mapper::job_to_row_values;

pub(super) async fn save_job(tx: &mut Transaction<'_, Sqlite>, job: &Job) -> Result<(), PortError> {
    let row = job_to_row_values(job)?;

    sqlx::query(
        r#"
        INSERT INTO jobs (
            id, project_id, title, kind, status, stage, progress_json, error_json, 
            created_at, updated_at, started_at, finished_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            status = excluded.status,
            stage = excluded.stage,
            progress_json = excluded.progress_json,
            error_json = excluded.error_json,
            updated_at = excluded.updated_at,
            started_at = excluded.started_at,
            finished_at = excluded.finished_at
        "#,
    )
    .bind(row.id)
    .bind(row.project_id)
    .bind(row.title)
    .bind(row.kind)
    .bind(row.status)
    .bind(row.stage)
    .bind(row.progress_json)
    .bind(row.error_json)
    .bind(row.created_at)
    .bind(row.updated_at)
    .bind(row.started_at)
    .bind(row.finished_at)
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to save job in tx: {}", e),
    })?;
    Ok(())
}

pub(super) async fn insert_job(
    tx: &mut Transaction<'_, Sqlite>,
    job: &Job,
) -> Result<(), PortError> {
    let row = job_to_row_values(job)?;

    sqlx::query(
        r#"
        INSERT INTO jobs (
            id, project_id, title, kind, status, stage, progress_json, error_json, 
            created_at, updated_at, started_at, finished_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(row.id)
    .bind(row.project_id)
    .bind(row.title)
    .bind(row.kind)
    .bind(row.status)
    .bind(row.stage)
    .bind(row.progress_json)
    .bind(row.error_json)
    .bind(row.created_at)
    .bind(row.updated_at)
    .bind(row.started_at)
    .bind(row.finished_at)
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to insert job in tx: {}", e),
    })?;
    Ok(())
}

pub(super) async fn update_job(
    tx: &mut Transaction<'_, Sqlite>,
    job: &Job,
) -> Result<(), PortError> {
    let row = job_to_row_values(job)?;

    let result = sqlx::query(
        r#"
        UPDATE jobs SET
            title = ?,
            status = ?,
            stage = ?,
            progress_json = ?,
            error_json = ?,
            updated_at = ?,
            started_at = ?,
            finished_at = ?
        WHERE id = ?
        "#,
    )
    .bind(row.title)
    .bind(row.status)
    .bind(row.stage)
    .bind(row.progress_json)
    .bind(row.error_json)
    .bind(row.updated_at)
    .bind(row.started_at)
    .bind(row.finished_at)
    .bind(row.id)
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to update job in tx: {}", e),
    })?;

    if result.rows_affected() == 0 {
        return Err(PortError::NotFound {
            resource: "Job".to_string(),
        });
    }

    Ok(())
}
