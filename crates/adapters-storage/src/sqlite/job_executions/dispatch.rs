use domain::job::{Job, JobStatus};
use ports::PortError;
use sqlx::SqlitePool;

use crate::sqlite::{helpers::map_sqlite_error, unit_of_work::job_writes::update_job};

pub(super) async fn commit(pool: &SqlitePool, running: &Job) -> Result<(), PortError> {
    if running.status() != &JobStatus::Running || running.revision() < 2 {
        return Err(PortError::InvalidSource {
            message: "Dispatch requires a Running candidate".into(),
        });
    }
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| map_sqlite_error("begin_job_dispatch", e))?;
    let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs JOIN projects ON projects.id = jobs.project_id JOIN job_executions ON job_executions.job_id = jobs.id WHERE jobs.id = ? AND jobs.status = 'pending' AND jobs.revision = ? AND projects.status = 'Processing' AND projects.active_job_id = jobs.id)")
        .bind(running.id().to_string()).bind((running.revision() - 1) as i64).fetch_one(&mut *tx).await.map_err(|e| map_sqlite_error("check_dispatch_eligibility", e))?;
    if !eligible {
        return Err(PortError::Conflict {
            resource: format!("Job {}", running.id()),
            message: "Job cancelled, deleted, replaced or already dispatched".into(),
        });
    }
    update_job(&mut tx, running, running.revision() - 1).await?;
    tx.commit()
        .await
        .map_err(|e| map_sqlite_error("commit_job_dispatch", e))
}
