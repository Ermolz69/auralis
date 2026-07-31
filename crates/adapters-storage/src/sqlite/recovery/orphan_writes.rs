use sqlx::SqlitePool;

use ports::error::PortError;
use ports::recovery::{FailOrphanJobCommand, RecoveryApplyResult};

use crate::sqlite::helpers::{map_sqlite_error, serialize_enum, serialize_json};

fn map_recovery_sqlite_error(error: sqlx::Error) -> PortError {
    map_sqlite_error("recovery orphan write", error)
}

pub async fn commit_failed_orphan_job(
    pool: &SqlitePool,
    cmd: FailOrphanJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(map_recovery_sqlite_error)?;

    // Check that NO Processing project links to this job via active_job_id
    let has_linked_project: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM projects WHERE status = 'Processing' AND active_job_id = ? LIMIT 1",
    )
    .bind(cmd.job.id().to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_recovery_sqlite_error)?;

    if has_linked_project.is_some() {
        let _ = tx.rollback().await; // allow-fallback
        return Err(PortError::Conflict {
            resource: "jobs".to_string(),
            message: format!(
                "Orphan job {} was linked by a Processing project",
                cmd.job.id()
            ),
        });
    }

    let expected_job_status = serialize_enum(&cmd.expected_job_status, "expected_job_status")?;

    let rows = sqlx::query(
        "UPDATE jobs SET status = ?, updated_at = ?, progress_json = ?, error_json = ? 
         WHERE id = ? AND status = ?",
    )
    .bind(serialize_enum(cmd.job.status(), "job.status")?)
    .bind(cmd.job.updated_at())
    .bind(serialize_json(cmd.job.progress(), "job.progress")?)
    .bind(
        cmd.job
            .error()
            .map(|e| serialize_json(&e, "job.error"))
            .transpose()?,
    )
    .bind(cmd.job.id().to_string())
    .bind(&expected_job_status)
    .execute(&mut *tx)
    .await
    .map_err(map_recovery_sqlite_error)?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await; // allow-fallback
        let current_status: Option<String> =
            sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
                .bind(cmd.job.id().to_string())
                .fetch_optional(pool)
                .await
                .map_err(map_recovery_sqlite_error)?;

        let new_status = serialize_enum(cmd.job.status(), "job.status")?;
        if current_status == Some(new_status) {
            return Ok(RecoveryApplyResult::AlreadyApplied);
        } else {
            return Err(PortError::Conflict {
                resource: "jobs".to_string(),
                message: format!("Strict update failed for orphan job {}", cmd.job.id()),
            });
        }
    }

    tx.commit().await.map_err(map_recovery_sqlite_error)?;
    Ok(RecoveryApplyResult::Applied)
}
