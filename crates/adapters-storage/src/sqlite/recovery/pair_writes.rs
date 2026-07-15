use sqlx::SqlitePool;

use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, FailLegacyPairFallbackCommand, FailLegacyProjectWithoutJobCommand,
    FailProjectWithMissingLinkedJobCommand, ReconcileTerminalPairCommand, RecoveryApplyResult,
};

fn serialize_enum<T: serde::Serialize>(val: &T) -> Result<String, PortError> {
    serde_json::to_string(val)
        .map(|s| s.trim_matches('"').to_string())
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to serialize enum: {}", e),
        })
}

fn serialize_json<T: serde::Serialize>(val: &T) -> Result<String, PortError> {
    serde_json::to_string(val).map_err(|e| PortError::Unexpected {
        message: format!("Failed to serialize json: {}", e),
    })
}

pub async fn commit_failed_interrupted_pair(
    pool: &SqlitePool,
    cmd: FailInterruptedPairCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx: {}", e),
    })?;

    let expected_job_status = serialize_enum(&cmd.expected_job_status)?;

    let job_affected = sqlx::query(
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

    let expected_project_status = serialize_enum(&cmd.expected_project_status)?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let project_affected = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = ?, last_terminal_job_id = ?
         WHERE id = ? AND status = ? AND active_job_id = ? AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status())?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.active_job_id().map(|id| id.to_string()))
    .bind(cmd.project.last_terminal_job_id().map(|id| id.to_string()))
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(cmd.expected_active_job_id.to_string())
    .bind(expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if job_affected == 0 || project_affected == 0 {
        let current_job_status: Option<String> =
            sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
                .bind(cmd.job.id().to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: e.to_string(),
                })?;

        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let new_job_status = serialize_enum(cmd.job.status())?;
        let new_proj_status = serialize_enum(cmd.project.status())?;
        let new_active_job = cmd.project.active_job_id().map(|id| id.to_string());
        let new_last_terminal = cmd.project.last_terminal_job_id().map(|id| id.to_string());

        let job_ok = job_affected > 0 || current_job_status == Some(new_job_status);
        let proj_ok = project_affected > 0
            || current_project
                == Some((
                    new_proj_status.clone(),
                    new_active_job.clone(),
                    new_last_terminal.clone(),
                ));

        if !job_ok || !proj_ok {
            let _ = tx.rollback().await;
            return Err(PortError::Conflict {
                resource: "pair".to_string(),
                message: format!(
                    "Pair {}-{} state changed incompatibly",
                    cmd.project.id(),
                    cmd.job.id()
                ),
            });
        }

        if job_affected == 0 && project_affected == 0 {
            let _ = tx.rollback().await;
            return Ok(RecoveryApplyResult::AlreadyApplied);
        }
    }

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_reconciled_terminal_pair(
    pool: &SqlitePool,
    cmd: ReconcileTerminalPairCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx: {}", e),
    })?;

    // Check job still terminal and unchanged
    let expected_job_status = serialize_enum(&cmd.expected_job_status)?;

    let current_job_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
            .bind(cmd.job.id().to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

    if current_job_status != Some(expected_job_status.clone()) {
        let _ = tx.rollback().await;
        return Err(PortError::Conflict {
            resource: "projects".to_string(),
            message: format!(
                "Job {} status changed from expected terminal status",
                cmd.job.id()
            ),
        });
    }

    let expected_project_status = serialize_enum(&cmd.expected_project_status)?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let rows = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = ?, last_terminal_job_id = ?
         WHERE id = ? AND status = ? AND active_job_id = ? AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status())?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.active_job_id().map(|id| id.to_string()))
    .bind(cmd.project.last_terminal_job_id().map(|id| id.to_string()))
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(cmd.expected_active_job_id.to_string())
    .bind(expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await;
        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let new_status = serialize_enum(cmd.project.status())?;
        let expected_active = cmd.project.active_job_id().map(|id| id.to_string());
        let expected_last_terminal = cmd.project.last_terminal_job_id().map(|id| id.to_string());

        if current_project == Some((new_status, expected_active, expected_last_terminal)) {
            return Ok(RecoveryApplyResult::AlreadyApplied);
        } else {
            return Err(PortError::Conflict {
                resource: "projects".to_string(),
                message: format!(
                    "Strict update failed for reconciling project {}",
                    cmd.project.id()
                ),
            });
        }
    }

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_legacy_pair_fallback(
    pool: &SqlitePool,
    cmd: FailLegacyPairFallbackCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx: {}", e),
    })?;

    let expected_job_status = serialize_enum(&cmd.expected_job_status)?;

    let job_affected = sqlx::query(
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

    let expected_project_status = serialize_enum(&cmd.expected_project_status)?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let project_affected = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = ?
         WHERE id = ? AND status = ? AND active_job_id IS NULL AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status())?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.active_job_id().map(|id| id.to_string()))
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(&expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if job_affected == 0 || project_affected == 0 {
        let current_job_status: Option<String> =
            sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
                .bind(cmd.job.id().to_string())
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: e.to_string(),
                })?;

        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let new_job_status = serialize_enum(cmd.job.status())?;
        let new_proj_status = serialize_enum(cmd.project.status())?;
        let new_active_job = cmd.project.active_job_id().map(|id| id.to_string());
        let expected_last_terminal = cmd.project.last_terminal_job_id().map(|id| id.to_string());

        let job_ok = job_affected > 0 || current_job_status == Some(new_job_status);
        let proj_ok = project_affected > 0
            || current_project
                == Some((
                    new_proj_status.clone(),
                    new_active_job.clone(),
                    expected_last_terminal.clone(),
                ));

        if !job_ok || !proj_ok {
            let _ = tx.rollback().await;
            return Err(PortError::Conflict {
                resource: "pair".to_string(),
                message: format!(
                    "Legacy Pair {}-{} state changed incompatibly",
                    cmd.project.id(),
                    cmd.job.id()
                ),
            });
        }

        if job_affected == 0 && project_affected == 0 {
            let _ = tx.rollback().await;
            return Ok(RecoveryApplyResult::AlreadyApplied);
        }
    }

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_failed_project_with_missing_linked_job(
    pool: &SqlitePool,
    cmd: FailProjectWithMissingLinkedJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx: {}", e),
    })?;

    let expected_project_status = serialize_enum(&cmd.expected_project_status)?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let rows = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = NULL
         WHERE id = ? AND status = ? AND active_job_id = ? AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status())?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(cmd.expected_active_job_id.to_string())
    .bind(&expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await;
        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let new_status = serialize_enum(cmd.project.status())?;
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

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_failed_legacy_project_without_job(
    pool: &SqlitePool,
    cmd: FailLegacyProjectWithoutJobCommand,
) -> Result<RecoveryApplyResult, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
        message: format!("Failed to begin tx: {}", e),
    })?;

    let expected_project_status = serialize_enum(&cmd.expected_project_status)?;

    let expected_last_terminal = cmd
        .expected_last_terminal_job_id
        .clone()
        .map(|id| id.to_string());

    let rows = sqlx::query(
        "UPDATE projects SET status = ?, updated_at = ?, active_job_id = NULL
         WHERE id = ? AND status = ? AND active_job_id IS NULL AND last_terminal_job_id IS ?",
    )
    .bind(serialize_enum(cmd.project.status())?)
    .bind(cmd.project.updated_at())
    .bind(cmd.project.id().to_string())
    .bind(&expected_project_status)
    .bind(&expected_last_terminal)
    .execute(&mut *tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?
    .rows_affected();

    if rows == 0 {
        let _ = tx.rollback().await;
        let current_project: Option<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT status, active_job_id, last_terminal_job_id FROM projects WHERE id = ?",
        )
        .bind(cmd.project.id().to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let new_status = serialize_enum(cmd.project.status())?;
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
                    "Strict update failed for legacy project no job {}",
                    cmd.project.id()
                ),
            });
        }
    }

    tx.commit().await.map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    Ok(RecoveryApplyResult::Applied)
}
