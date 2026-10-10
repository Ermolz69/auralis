use domain::job::{JobId, JobStatus, TerminalOutcome};
use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::{ProjectId, ProjectStatus};
use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, ReconcileTerminalPairCommand, RecoveryApplyResult,
};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::sqlite::helpers::{map_sqlite_error, serialize_enum, serialize_json};
use crate::sqlite::unit_of_work::outbox_writes::save_outbox_message;

fn storage_error(error: sqlx::Error) -> PortError {
    map_sqlite_error("recovery pair write", error)
}

async fn verify_project(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &ProjectId,
    revision: u64,
    status: &ProjectStatus,
    active_job_id: &JobId,
    last_terminal_job_id: Option<&JobId>,
) -> Result<(), PortError> {
    let valid: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ? AND revision = ?
         AND status = ? AND active_job_id = ? AND last_terminal_job_id IS ?)",
    )
    .bind(project_id.to_string())
    .bind(revision as i64)
    .bind(serialize_enum(status, "expected_project_status")?)
    .bind(active_job_id.to_string())
    .bind(last_terminal_job_id.map(ToString::to_string))
    .fetch_one(&mut **tx)
    .await
    .map_err(storage_error)?;
    if valid != 1 {
        return Err(PortError::Conflict {
            resource: "Project recovery".into(),
            message: "Project revision, status, or job identity changed".into(),
        });
    }
    Ok(())
}

async fn has_unresolved_terminal_intent(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &ProjectId,
    job_id: &JobId,
) -> Result<bool, PortError> {
    let existing: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM outbox_messages
         WHERE kind = 'handle_terminal_job_state' AND status != 'done'
           AND aggregate_type = 'project' AND aggregate_id = ?
           AND json_valid(payload_json)
           AND json_extract(payload_json, '$.project_id') = ?
           AND json_extract(payload_json, '$.job_id') = ?)",
    )
    .bind(project_id.to_string())
    .bind(project_id.to_string())
    .bind(job_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(storage_error)?;
    Ok(existing == 1)
}

async fn enqueue_terminal_intent(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &ProjectId,
    job_id: &JobId,
    outcome: TerminalOutcome,
) -> Result<RecoveryApplyResult, PortError> {
    if has_unresolved_terminal_intent(tx, project_id, job_id).await? {
        return Ok(RecoveryApplyResult::AlreadyApplied);
    }
    let message = OutboxMessage::new(OutboxPayload::HandleTerminalJobState {
        job_id: job_id.clone(),
        project_id: project_id.clone(),
        outcome,
    });
    save_outbox_message(tx, &message).await?;
    Ok(RecoveryApplyResult::Applied)
}

pub async fn commit_failed_interrupted_pair(
    pool: &SqlitePool,
    cmd: FailInterruptedPairCommand,
) -> Result<RecoveryApplyResult, PortError> {
    if cmd.job.status() != &JobStatus::Failed || cmd.job.project_id() != cmd.project.id() {
        return Err(PortError::Conflict {
            resource: "Interrupted job recovery".into(),
            message: "Recovery result does not match the project or failure outcome".into(),
        });
    }
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(storage_error)?;
    verify_project(
        &mut tx,
        cmd.project.id(),
        cmd.project.revision(),
        &cmd.expected_project_status,
        &cmd.expected_active_job_id,
        cmd.expected_last_terminal_job_id.as_ref(),
    )
    .await?;

    let affected = sqlx::query(
        "UPDATE jobs SET status = ?, updated_at = ?, progress_json = ?, error_json = ?,
         revision = ?, finished_at = ?
         WHERE id = ? AND project_id = ? AND status = ? AND revision = ?",
    )
    .bind(serialize_enum(cmd.job.status(), "job.status")?)
    .bind(cmd.job.updated_at())
    .bind(serialize_json(cmd.job.progress(), "job.progress")?)
    .bind(
        cmd.job
            .error()
            .map(|error| serialize_json(&error, "job.error"))
            .transpose()?,
    )
    .bind(cmd.job.revision() as i64)
    .bind(cmd.job.finished_at())
    .bind(cmd.job.id().to_string())
    .bind(cmd.project.id().to_string())
    .bind(serialize_enum(
        &cmd.expected_job_status,
        "expected_job_status",
    )?)
    .bind(cmd.expected_job_revision as i64)
    .execute(&mut *tx)
    .await
    .map_err(storage_error)?
    .rows_affected();

    if affected == 0 {
        let already_failed: i64 = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM jobs WHERE id = ? AND project_id = ?
             AND status = 'failed' AND revision = ?)",
        )
        .bind(cmd.job.id().to_string())
        .bind(cmd.project.id().to_string())
        .bind(cmd.job.revision() as i64)
        .fetch_one(&mut *tx)
        .await
        .map_err(storage_error)?;
        if already_failed != 1 {
            return Err(PortError::Conflict {
                resource: "Interrupted job recovery".into(),
                message: "Job status or revision changed".into(),
            });
        }
    }

    let result = enqueue_terminal_intent(
        &mut tx,
        cmd.project.id(),
        cmd.job.id(),
        TerminalOutcome::Failed,
    )
    .await?;
    tx.commit().await.map_err(storage_error)?;
    Ok(result)
}

pub async fn commit_reconciled_terminal_pair(
    pool: &SqlitePool,
    cmd: ReconcileTerminalPairCommand,
) -> Result<RecoveryApplyResult, PortError> {
    if cmd.job.status() != &cmd.expected_job_status || cmd.job.project_id() != cmd.project.id() {
        return Err(PortError::Conflict {
            resource: "Terminal job recovery".into(),
            message: "Recovery result does not match the project or job snapshot".into(),
        });
    }
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(storage_error)?;
    verify_project(
        &mut tx,
        cmd.project.id(),
        cmd.project.revision(),
        &cmd.expected_project_status,
        &cmd.expected_active_job_id,
        cmd.expected_last_terminal_job_id.as_ref(),
    )
    .await?;

    let current_job: i64 = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM jobs WHERE id = ? AND project_id = ?
         AND status = ? AND revision = ?)",
    )
    .bind(cmd.job.id().to_string())
    .bind(cmd.project.id().to_string())
    .bind(serialize_enum(
        &cmd.expected_job_status,
        "expected_job_status",
    )?)
    .bind(cmd.job.revision() as i64)
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    if current_job != 1 {
        return Err(PortError::Conflict {
            resource: "Terminal job recovery".into(),
            message: "Job identity, revision, or terminal status changed".into(),
        });
    }
    let outcome = match cmd.expected_job_status {
        JobStatus::Completed => TerminalOutcome::Completed,
        JobStatus::Failed => TerminalOutcome::Failed,
        JobStatus::Cancelled => TerminalOutcome::Cancelled,
        _ => {
            return Err(PortError::Conflict {
                resource: "Terminal job recovery".into(),
                message: "Job is not terminal".into(),
            });
        }
    };
    let result = enqueue_terminal_intent(&mut tx, cmd.project.id(), cmd.job.id(), outcome).await?;
    tx.commit().await.map_err(storage_error)?;
    Ok(result)
}
