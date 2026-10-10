use domain::job::{Job, JobId};
use ports::{
    PortError,
    job_execution::{ExecutionDescriptor, JobExecutionStore, PendingExecution},
    transaction::CommitPipelineStart,
};

#[derive(Clone)]
pub struct SqliteJobExecutionStore {
    pub(super) pool: sqlx::SqlitePool,
}

impl SqliteJobExecutionStore {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl JobExecutionStore for SqliteJobExecutionStore {
    async fn admit_pipeline(
        &self,
        start: CommitPipelineStart,
        descriptor: ExecutionDescriptor,
        capacity: usize,
    ) -> Result<(), PortError> {
        super::admission::commit(&self.pool, start, descriptor, capacity).await
    }

    async fn list_pending(&self, limit: usize) -> Result<Vec<PendingExecution>, PortError> {
        let rows = sqlx::query_as::<_, super::super::job_row::JobRow>(
            "SELECT jobs.* FROM jobs INNER JOIN job_executions ON job_executions.job_id = jobs.id WHERE jobs.status = 'pending' ORDER BY jobs.created_at, jobs.id LIMIT ?"
        ).bind(limit as i64).fetch_all(&self.pool).await.map_err(|e| super::super::helpers::map_sqlite_error("load_job_queue", e))?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let job = super::super::job_mapper::row_to_job(row)?;
            let descriptor = match self.descriptor(job.id()).await {
                Ok(Some(descriptor)) => Ok(descriptor),
                Ok(None) => continue,
                Err(error @ PortError::InvalidStoredData { .. }) => Err(error),
                Err(error) => return Err(error),
            };
            entries.push(PendingExecution { job, descriptor });
        }
        Ok(entries)
    }

    async fn descriptor(&self, job_id: &JobId) -> Result<Option<ExecutionDescriptor>, PortError> {
        let json: Option<String> =
            sqlx::query_scalar("SELECT descriptor_json FROM job_executions WHERE job_id = ?")
                .bind(job_id.to_string())
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    super::super::helpers::map_sqlite_error("load_execution_descriptor", e)
                })?;
        json.map(|json| super::mapping::decode(job_id, &json))
            .transpose()
    }

    async fn start(&self, job: &Job) -> Result<(), PortError> {
        super::dispatch::commit(&self.pool, job).await
    }
}
