use sqlx::SqlitePool;

use ports::error::PortError;
use ports::recovery::{FailOrphanJobCommand, RecoveryApplyResult};

fn serialize_enum<T: serde::Serialize>(val: &T) -> Result<String, PortError> {
    serde_json::to_string(val)
        .map(|s| s.trim_matches('"').to_string())
        .map_err(|e| PortError::Storage {
            operation: "serialize_enum",
            message: e.to_string(),
        })
}

fn serialize_json<T: serde::Serialize>(val: &T) -> Result<String, PortError> {
    serde_json::to_string(val).map_err(|e| PortError::Storage {
        operation: "serialize_json",
        message: e.to_string(),
    })
}

pub async fn commit_failed_orphan_job(
    pool: &SqlitePool,
    cmd: FailOrphanJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to begin tx", e))?;

    // Check that NO Processing project links to this job via active_job_id
    let has_linked_project: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM projects WHERE status = 'Processing' AND active_job_id = ? LIMIT 1",
    )
    .bind(cmd.job.id().to_string())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;

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

    let expected_job_status = serialize_enum(&cmd.expected_job_status)?;

    let rows = sqlx::query(
        "UPDATE jobs SET status = ?, updated_at = ?, progress_json = ?, error_json = ? 
         WHERE id = ? AND status = ?",
    )
    .bind(serialize_enum(cmd.job.status())?)
    .bind(cmd.job.updated_at())
    .bind(serialize_json(cmd.job.progress())?)
    .bind(cmd.job.error().map(|e| serialize_json(&e)).transpose()?)
    .bind(cmd.job.id().to_string())
    .bind(&expected_job_status)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await; // allow-fallback
        let current_status: Option<String> =
            sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
                .bind(cmd.job.id().to_string())
                .fetch_optional(pool)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: e.to_string(),
                })?;

        let new_status = serialize_enum(cmd.job.status())?;
        if current_status == Some(new_status) {
            return Ok(RecoveryApplyResult::AlreadyApplied);
        } else {
            return Err(PortError::Conflict {
                resource: "jobs".to_string(),
                message: format!("Strict update failed for orphan job {}", cmd.job.id()),
            });
        }
    }

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}
