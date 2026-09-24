use async_trait::async_trait;
use domain::job::{Job, JobKind, JobStatus};
use ports::active_translation_host_job::ActiveTranslationHostJob;
use ports::error::PortError;
use ports::translation_host_job::TranslationHostJobStore;
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

use super::helpers::map_sqlite_error;
use super::job_mapper::{job_to_row_values, row_to_job};
use super::job_row::JobRow;

pub struct SqliteTranslationHostJobStore {
    pool: SqlitePool,
}

impl SqliteTranslationHostJobStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TranslationHostJobStore for SqliteTranslationHostJobStore {
    async fn create_pending(
        &self,
        job: Job,
        translation_id: Uuid,
        run_id: Uuid,
    ) -> Result<Job, PortError> {
        if job.kind() != &JobKind::Translation
            || job.status() != &JobStatus::Pending
            || job.stage().is_some()
            || job.revision() != 1
            || job.started_at().is_some()
            || job.finished_at().is_some()
        {
            return Err(PortError::InvalidSource {
                message: "host translation job must be pending, translation kind, and stage-free"
                    .into(),
            });
        }
        let values = job_to_row_values(&job)?;
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| map_sqlite_error("begin_translation_host_job", error))?;
        let inserted = sqlx::query(
            "INSERT INTO jobs (
                id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                created_at, updated_at, started_at, finished_at
            ) SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
              FROM translation_run_intents AS intent
              JOIN project_translations AS link ON link.translation_id = intent.translation_id
             WHERE intent.run_id = ? AND intent.translation_id = ?
               AND link.active_run_id = intent.run_id AND link.project_id = ?",
        )
        .bind(&values.id)
        .bind(values.revision)
        .bind(&values.project_id)
        .bind(&values.title)
        .bind(&values.kind)
        .bind(&values.status)
        .bind(&values.stage)
        .bind(&values.progress_json)
        .bind(&values.error_json)
        .bind(&values.created_at)
        .bind(&values.updated_at)
        .bind(&values.started_at)
        .bind(&values.finished_at)
        .bind(run_id.to_string())
        .bind(translation_id.to_string())
        .bind(&values.project_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| conflict_or_storage("create_translation_host_job", error))?;
        if inserted.rows_affected() != 1 {
            return Err(PortError::Conflict {
                resource: "translation run".into(),
                message: "run is not the active frozen intent of this project translation".into(),
            });
        }
        sqlx::query(
            "INSERT INTO translation_host_jobs (job_id, run_id, created_at) VALUES (?, ?, ?)",
        )
        .bind(&values.id)
        .bind(run_id.to_string())
        .bind(&values.created_at)
        .execute(&mut *transaction)
        .await
        .map_err(|error| conflict_or_storage("associate_translation_host_job", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlite_error("commit_translation_host_job", error))?;
        Ok(job)
    }

    async fn active_for_run(&self, run_id: Uuid) -> Result<Option<Job>, PortError> {
        sqlx::query_as::<_, JobRow>(
            "SELECT job.id, job.revision, job.project_id, job.title, job.kind, job.status,
                    job.stage, job.progress_json, job.error_json, job.created_at, job.updated_at,
                    job.started_at, job.finished_at
               FROM translation_host_jobs AS host
               JOIN jobs AS job ON job.id = host.job_id
              WHERE host.run_id = ? AND host.ended_at IS NULL",
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("get_active_translation_host_job", error))?
        .map(row_to_job)
        .transpose()
    }

    async fn list_active(&self) -> Result<Vec<ActiveTranslationHostJob>, PortError> {
        let rows = sqlx::query(
            "SELECT job.id, job.revision, job.project_id, job.title, job.kind, job.status,
                    job.stage, job.progress_json, job.error_json, job.created_at, job.updated_at,
                    job.started_at, job.finished_at, host.run_id AS host_run_id
               FROM translation_host_jobs AS host
               JOIN jobs AS job ON job.id = host.job_id
              WHERE host.ended_at IS NULL ORDER BY host.created_at, host.job_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlite_error("list_active_translation_host_jobs", error))?;
        rows.into_iter()
            .map(|row| {
                let job = JobRow::from_row(&row)
                    .map_err(|error| map_sqlite_error("decode_translation_host_job", error))
                    .and_then(row_to_job)?;
                let raw: String = row
                    .try_get("host_run_id")
                    .map_err(|error| map_sqlite_error("decode_translation_host_run", error))?;
                let run_id = Uuid::parse_str(&raw).map_err(|_| PortError::InvalidStoredData {
                    entity_type: "translation host job".into(),
                    entity_id: job.id().to_string(),
                    field: "run_id".into(),
                    message: "invalid stored run ID".into(),
                })?;
                Ok(ActiveTranslationHostJob { job, run_id })
            })
            .collect()
    }

    async fn finish(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        if job.kind() != &JobKind::Translation
            || job.stage().is_some()
            || !matches!(
                job.status(),
                JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
            )
            || job.finished_at().is_none()
            || job.revision() != expected_revision.saturating_add(1)
        {
            return Err(PortError::InvalidSource {
                message: "host translation job must have a new terminal revision".into(),
            });
        }
        let values = job_to_row_values(job)?;
        let mut transaction = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| map_sqlite_error("begin_finish_translation_host_job", error))?;
        let updated = sqlx::query(
            "UPDATE jobs SET revision = ?, status = ?, progress_json = ?, error_json = ?,
                    updated_at = ?, finished_at = ?
              WHERE id = ? AND revision = ? AND project_id = ? AND kind = ?
                AND status IN ('pending', 'running', 'cancelling')
                AND EXISTS (SELECT 1 FROM translation_host_jobs AS host
                             WHERE host.job_id = jobs.id AND host.ended_at IS NULL)",
        )
        .bind(values.revision)
        .bind(&values.status)
        .bind(&values.progress_json)
        .bind(&values.error_json)
        .bind(&values.updated_at)
        .bind(&values.finished_at)
        .bind(&values.id)
        .bind(
            i64::try_from(expected_revision).map_err(|_| PortError::InvalidSource {
                message: "invalid expected host job revision".into(),
            })?,
        )
        .bind(&values.project_id)
        .bind(&values.kind)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("finish_translation_host_job", error))?;
        if updated.rows_affected() != 1 {
            return Err(PortError::Conflict {
                resource: "translation host job".into(),
                message: "active association or expected job revision changed".into(),
            });
        }
        let ended = sqlx::query(
            "UPDATE translation_host_jobs SET ended_at = ? WHERE job_id = ? AND ended_at IS NULL",
        )
        .bind(&values.finished_at)
        .bind(&values.id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlite_error("end_translation_host_job_association", error))?;
        if ended.rows_affected() != 1 {
            return Err(PortError::Conflict {
                resource: "translation host job".into(),
                message: "active association changed".into(),
            });
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlite_error("commit_finish_translation_host_job", error))
    }
}

fn conflict_or_storage(operation: &'static str, error: sqlx::Error) -> PortError {
    if let sqlx::Error::Database(ref database_error) = error
        && database_error.is_unique_violation()
    {
        return PortError::Conflict {
            resource: "translation host job".into(),
            message: "job ID already exists or run already has an active host job".into(),
        };
    }
    map_sqlite_error(operation, error)
}
