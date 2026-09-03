use sqlx::SqlitePool;

use ports::error::PortError;
use ports::recovery::{
    FailProjectWithMissingLinkedJobCommand, FailProjectWithoutActiveJobCommand, RecoveryApplyResult,
};

use crate::sqlite::helpers::{map_sqlite_error, serialize_enum};

fn map_recovery_sqlite_error(error: sqlx::Error) -> PortError {
    map_sqlite_error("recovery project write", error)
}

pub async fn commit_failed_project_with_missing_linked_job(
    pool: &SqlitePool,
    cmd: FailProjectWithMissingLinkedJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(map_recovery_sqlite_error)?;

    let expected_project_status =
        serialize_enum(&cmd.expected_project_status, "expected_project_status")?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let rows = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = NULL, revision = revision + 1
         WHERE id = ? AND status = ? AND active_job_id = ? AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status(), "project.status")?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(cmd.expected_active_job_id.to_string())
    .bind(&expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(map_recovery_sqlite_error)?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await; // allow-fallback
        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(pool)
        .await
        .map_err(map_recovery_sqlite_error)?;

        let new_status = serialize_enum(cmd.project.status(), "project.status")?;
        let expected_last_terminal = cmd
            .expected_last_terminal_job_id
            .clone()
            .map(|id| id.to_string());
        if current_project == Some((new_status, None, expected_last_terminal)) {
            return Ok(RecoveryApplyResult::AlreadyApplied);
        } else {
            return Err(PortError::Conflict {
                resource: "projects".to_string(),
                message: format!(
                    "Strict update failed for project with missing job {}",
                    cmd.project.id()
                ),
            });
        }
    }

    tx.commit().await.map_err(map_recovery_sqlite_error)?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_failed_project_without_active_job(
    pool: &SqlitePool,
    cmd: FailProjectWithoutActiveJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(map_recovery_sqlite_error)?;

    let expected_project_status =
        serialize_enum(&cmd.expected_project_status, "expected_project_status")?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let rows = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = NULL, revision = revision + 1
         WHERE id = ? AND status = ? AND active_job_id IS NULL AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status(), "project.status")?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(&expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(map_recovery_sqlite_error)?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await; // allow-fallback
        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(pool)
        .await
        .map_err(map_recovery_sqlite_error)?;

        let new_status = serialize_enum(cmd.project.status(), "project.status")?;
        let expected_last_terminal = cmd
            .expected_last_terminal_job_id
            .clone()
            .map(|id| id.to_string());
        if current_project == Some((new_status, None, expected_last_terminal)) {
            return Ok(RecoveryApplyResult::AlreadyApplied);
        } else {
            return Err(PortError::Conflict {
                resource: "projects".to_string(),
                message: format!(
                    "Strict update failed for project without an active job {}",
                    cmd.project.id()
                ),
            });
        }
    }

    tx.commit().await.map_err(map_recovery_sqlite_error)?;
    Ok(RecoveryApplyResult::Applied)
}
