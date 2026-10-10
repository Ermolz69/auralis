use ports::{PortError, job_execution::ExecutionDescriptor, transaction::CommitPipelineStart};
use sqlx::SqlitePool;

use crate::sqlite::{
    helpers::map_sqlite_error,
    unit_of_work::{job_writes::insert_job, project_writes::update_project},
};

pub(super) async fn commit(
    pool: &SqlitePool,
    start: CommitPipelineStart,
    descriptor: ExecutionDescriptor,
    capacity: usize,
) -> Result<(), PortError> {
    start.validate()?;
    descriptor.validate()?;
    if capacity == 0
        || capacity > 100_000
        || descriptor.project_id != *start.project.id()
        || start.project.active_job_id() != Some(start.job.id())
        || start.job.kind() != &domain::job::JobKind::Dubbing
    {
        return Err(PortError::InvalidSource {
            message: "Invalid queue admission".into(),
        });
    }
    let json = serde_json::to_string(&descriptor).map_err(|e| PortError::InvalidSource {
        message: e.to_string(),
    })?;
    if json.len() > 16_384 {
        return Err(PortError::InvalidSource {
            message: "Execution descriptor is too large".into(),
        });
    }
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| map_sqlite_error("begin_job_admission", e))?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM job_executions JOIN jobs ON jobs.id = job_executions.job_id WHERE jobs.status = 'pending'")
        .fetch_one(&mut *tx).await.map_err(|e| map_sqlite_error("count_job_queue", e))?;
    if count >= capacity as i64 {
        return Err(PortError::QueueFull { capacity });
    }
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE project_id = ? AND kind IN ('dubbing', 'Dubbing') AND status IN ('pending', 'running', 'cancelling', 'Pending', 'Running', 'Cancelling'))")
        .bind(start.project.id().to_string()).fetch_one(&mut *tx).await.map_err(|e| map_sqlite_error("check_project_admission", e))?;
    if active {
        return Err(PortError::Conflict {
            resource: format!("Project {}", start.project.id()),
            message: "Project already has an active pipeline job".into(),
        });
    }
    update_project(&mut tx, &start.project).await?;
    insert_job(&mut tx, &start.job).await?;
    sqlx::query("INSERT INTO job_executions(job_id, descriptor_json) VALUES (?, ?)")
        .bind(start.job.id().to_string())
        .bind(json)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_sqlite_error("insert_execution_descriptor", e))?;
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("commit_job_admission", e))
}
