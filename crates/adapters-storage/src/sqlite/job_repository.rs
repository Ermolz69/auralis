use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::job::{Job, JobId};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::JobRepository;

use super::job_mapper::{job_to_row_values, row_to_job};
use super::job_row::JobRow;

pub struct SqliteJobRepository {
    pool: SqlitePool,
}

impl SqliteJobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobRepository for SqliteJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        let values = job_to_row_values(&job)?;

        sqlx::query(
            r#"
            INSERT INTO jobs (
                id, project_id, title, kind, status, stage, progress_json, error_json,
                created_at, updated_at, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(values.id)
        .bind(values.project_id)
        .bind(values.title)
        .bind(values.kind)
        .bind(values.status)
        .bind(values.stage)
        .bind(values.progress_json)
        .bind(values.error_json)
        .bind(values.created_at)
        .bind(values.updated_at)
        .bind(values.started_at)
        .bind(values.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to create job: {}", e),
        })?;

        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch job: {}", e),
        })?;

        row.map(row_to_job).transpose()
    }

    async fn save(&self, job: &Job) -> Result<(), PortError> {
        let values = job_to_row_values(job)?;

        sqlx::query(
            r#"
            INSERT INTO jobs (
                id, project_id, title, kind, status, stage, progress_json, error_json,
                created_at, updated_at, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                title = excluded.title,
                kind = excluded.kind,
                status = excluded.status,
                stage = excluded.stage,
                progress_json = excluded.progress_json,
                error_json = excluded.error_json,
                updated_at = excluded.updated_at,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at
            "#,
        )
        .bind(values.id)
        .bind(values.project_id)
        .bind(values.title)
        .bind(values.kind)
        .bind(values.status)
        .bind(values.stage)
        .bind(values.progress_json)
        .bind(values.error_json)
        .bind(values.created_at)
        .bind(values.updated_at)
        .bind(values.started_at)
        .bind(values.finished_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to save job: {}", e),
        })?;

        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE project_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list jobs by project: {}", e),
        })?;

        rows.into_iter().map(row_to_job).collect()
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE status IN ('pending', 'running', 'Pending', 'Running')
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list active jobs: {}", e),
        })?;

        rows.into_iter().map(row_to_job).collect()
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list recent jobs: {}", e),
        })?;

        rows.into_iter().map(row_to_job).collect()
    }
}
