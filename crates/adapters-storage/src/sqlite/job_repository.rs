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
                id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                created_at, updated_at, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(values.id)
        .bind(values.revision)
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
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e
                && db_err.is_unique_violation()
            {
                return PortError::Conflict {
                    resource: "Job".to_string(),
                    message: format!("Job with id {} already exists", job.id()),
                };
            }
            crate::sqlite::helpers::map_sqlite_error("create_job", e)
        })?;

        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let row = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("get_job", e))?;

        row.map(row_to_job).transpose()
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        let values = job_to_row_values(job)?;

        let result = sqlx::query(
            r#"
            UPDATE jobs SET
                project_id = ?,
                title = ?,
                kind = ?,
                status = ?,
                stage = ?,
                progress_json = ?,
                error_json = ?,
                updated_at = ?,
                started_at = ?,
                finished_at = ?,
                revision = ?
            WHERE id = ? AND revision = ?
            "#,
        )
        .bind(values.project_id)
        .bind(values.title)
        .bind(values.kind)
        .bind(values.status)
        .bind(values.stage)
        .bind(values.progress_json)
        .bind(values.error_json)
        .bind(values.updated_at)
        .bind(values.started_at)
        .bind(values.finished_at)
        .bind(values.revision)
        .bind(values.id)
        .bind(i64::try_from(expected_revision).unwrap_or(-1))
        .execute(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("save_job", e))?;

        if result.rows_affected() == 0 {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Optimistic concurrency conflict for job id {}", job.id()),
            });
        }

        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE project_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("list_jobs_by_project", e))?;

        rows.into_iter().map(row_to_job).collect()
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE status IN (
                'pending', 'running', 'cancelling',
                'Pending', 'Running', 'Cancelling'
            )
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("list_active_jobs", e))?;

        rows.into_iter().map(row_to_job).collect()
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("list_recent_jobs", e))?;

        rows.into_iter().map(row_to_job).collect()
    }
}

#[async_trait]
impl ports::job_query::JobQueryPort for SqliteJobRepository {
    async fn list_jobs_snapshot(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ports::job_scheduler::ScheduledJob>, PortError> {
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE project_id = ?
            "#,
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("list_jobs_snapshot", e))?;

        let mut jobs = Vec::with_capacity(rows.len());
        for row in rows {
            let job = row_to_job(row)?;
            let snap = job.to_snapshot();
            jobs.push(ports::job_scheduler::ScheduledJob {
                id: snap.id,
                kind: snap.kind,
                revision: snap.revision,
                project_id: Some(snap.project_id),
                title: snap.title,
                status: snap.status,
                stage: snap.stage,
                progress: snap.progress,
                error: snap.error.map(|e| e.message),
                created_at: snap.created_at,
                updated_at: snap.updated_at,
            });
        }

        Ok(jobs)
    }

    async fn list_job_history_page(
        &self,
        cursor: Option<&ports::job_query::JobHistoryCursor>,
        limit: usize,
    ) -> Result<ports::job_query::JobHistoryPage, PortError> {
        let page_size = limit.clamp(1, ports::job_query::MAX_JOB_HISTORY_PAGE_SIZE);
        let cursor_created_at = cursor.map(|value| value.created_at.to_rfc3339());
        let cursor_job_id = cursor.map(|value| value.job_id.to_string());
        let rows = sqlx::query_as::<_, JobRow>(
            r#"
            SELECT id, revision, project_id, title, kind, status, stage, progress_json, error_json,
                   created_at, updated_at, started_at, finished_at
            FROM jobs
            WHERE status IN (
                'completed', 'failed', 'cancelled',
                'Completed', 'Failed', 'Cancelled'
            )
              AND (
                ? IS NULL
                OR created_at < ?
                OR (created_at = ? AND id < ?)
              )
            ORDER BY created_at DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(cursor_created_at.clone())
        .bind(cursor_created_at.clone())
        .bind(cursor_created_at)
        .bind(cursor_job_id)
        .bind((page_size + 1) as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("list_job_history_page", error)
        })?;

        let has_more = rows.len() > page_size;
        let mut jobs = Vec::with_capacity(page_size.min(rows.len()));
        for row in rows.into_iter().take(page_size) {
            jobs.push(job_to_scheduled(row_to_job(row)?));
        }
        let next_cursor = if has_more {
            jobs.last().map(|last| ports::job_query::JobHistoryCursor {
                created_at: last.created_at,
                job_id: last.id.clone(),
            })
        } else {
            None
        };

        Ok(ports::job_query::JobHistoryPage { jobs, next_cursor })
    }
}

fn job_to_scheduled(job: Job) -> ports::job_scheduler::ScheduledJob {
    let snap = job.to_snapshot();
    ports::job_scheduler::ScheduledJob {
        id: snap.id,
        kind: snap.kind,
        revision: snap.revision,
        project_id: Some(snap.project_id),
        title: snap.title,
        status: snap.status,
        stage: snap.stage,
        progress: snap.progress,
        error: snap.error.map(|error| error.message),
        created_at: snap.created_at,
        updated_at: snap.updated_at,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use ports::job_query::JobQueryPort;
    use ports::repository::{JobRepository, ProjectRepository};
    use std::collections::HashSet;

    #[tokio::test]
    async fn history_cursor_reads_more_than_one_hundred_without_duplicates() {
        let directory = tempfile::tempdir().unwrap();
        let pool = crate::sqlite::connect_sqlite(directory.path().join("history.sqlite"))
            .await
            .unwrap();
        let project_repo = crate::sqlite::SqliteProjectRepository::new(pool.clone());
        let project = project_repo
            .create(domain::project::Project::new("History".into()).unwrap())
            .await
            .unwrap();
        let repository = SqliteJobRepository::new(pool);

        for index in 0..205 {
            let mut job = Job::new(
                project.id().clone(),
                format!("Terminal {index}"),
                domain::job::JobKind::Dubbing,
            );
            job.cancel().unwrap();
            repository.create(job).await.unwrap();
        }
        let mut active = Job::new(
            project.id().clone(),
            "Active".into(),
            domain::job::JobKind::Dubbing,
        );
        active.start().unwrap();
        let active_id = active.id().clone();
        repository.create(active).await.unwrap();

        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = repository
                .list_job_history_page(cursor.as_ref(), 37)
                .await
                .unwrap();
            ids.extend(page.jobs.iter().map(|job| job.id.clone()));
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }

        assert_eq!(ids.len(), 205);
        assert_eq!(ids.iter().cloned().collect::<HashSet<_>>().len(), 205);
        assert!(!ids.contains(&active_id));
    }
}
