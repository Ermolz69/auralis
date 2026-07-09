use async_trait::async_trait;
use sqlx::{Row, sqlite::SqlitePool};
use std::str::FromStr;

use domain::job::JobSnapshot;
use domain::job::{Job, JobId};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::JobRepository;

pub struct SqliteJobRepository {
    pool: SqlitePool,
}

impl SqliteJobRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn connect<P: AsRef<std::path::Path>>(db_path: P) -> Result<Self, PortError> {
        let db_url = format!("sqlite:{}?mode=rwc", db_path.as_ref().display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(&db_url)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to connect to sqlite db: {}", e),
            })?;

        let repo = Self { pool };
        repo.init().await?;
        Ok(repo)
    }

    pub async fn init(&self) -> Result<(), PortError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                stage TEXT,
                progress_json TEXT NOT NULL,
                error_json TEXT,
                created_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_project_id ON jobs(project_id);
            CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to init jobs table: {}", e),
        })?;
        Ok(())
    }
}

#[async_trait]
impl JobRepository for SqliteJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        self.save(&job).await?;
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, project_id, kind, status, stage, progress_json, error_json, 
                created_at, started_at, finished_at
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

        if let Some(row) = row {
            let snapshot = row_to_snapshot(row)?;
            let job = Job::from_snapshot(snapshot);
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, job: &Job) -> Result<(), PortError> {
        let snapshot = job.to_snapshot();

        let kind_json = serde_json::to_string(&snapshot.kind).unwrap_or_default();
        let status_json = serde_json::to_string(&snapshot.status).unwrap_or_default();
        let stage_json = snapshot
            .stage
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_default());
        let progress_json = serde_json::to_string(&snapshot.progress).unwrap_or_default();
        let error_json = snapshot
            .error
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_default());

        let started_at_str = snapshot.started_at.map(|d| d.to_rfc3339());
        let finished_at_str = snapshot.finished_at.map(|d| d.to_rfc3339());

        sqlx::query(
            r#"
            INSERT INTO jobs (
                id, project_id, kind, status, stage, progress_json, error_json, 
                created_at, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                kind = excluded.kind,
                status = excluded.status,
                stage = excluded.stage,
                progress_json = excluded.progress_json,
                error_json = excluded.error_json,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at
            "#,
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.project_id.to_string())
        .bind(kind_json)
        .bind(status_json)
        .bind(stage_json)
        .bind(progress_json)
        .bind(error_json)
        .bind(snapshot.created_at.to_rfc3339())
        .bind(started_at_str)
        .bind(finished_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to save job: {}", e),
        })?;

        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, project_id, kind, status, stage, progress_json, error_json, 
                created_at, started_at, finished_at
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

        let mut jobs = Vec::new();
        for row in rows {
            let snapshot = row_to_snapshot(row)?;
            let job = Job::from_snapshot(snapshot);
            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let pending_json = serde_json::to_string(&domain::job::JobStatus::Pending).unwrap();
        let running_json = serde_json::to_string(&domain::job::JobStatus::Running).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT 
                id, project_id, kind, status, stage, progress_json, error_json, 
                created_at, started_at, finished_at
            FROM jobs 
            WHERE status IN (?, ?)
            ORDER BY created_at ASC
            "#,
        )
        .bind(pending_json)
        .bind(running_json)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list active jobs: {}", e),
        })?;

        let mut jobs = Vec::new();
        for row in rows {
            let snapshot = row_to_snapshot(row)?;
            let job = Job::from_snapshot(snapshot);
            jobs.push(job);
        }

        Ok(jobs)
    }
}

fn row_to_snapshot(row: sqlx::sqlite::SqliteRow) -> Result<JobSnapshot, PortError> {
    let id_str: String = row.try_get("id").map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    let id = JobId::from_str(&id_str).map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;

    let project_id_str: String = row.try_get("project_id").map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;
    let project_id = ProjectId::from_str(&project_id_str).map_err(|e| PortError::Unexpected {
        message: e.to_string(),
    })?;

    let kind_json: String = row.try_get("kind").unwrap_or_default();
    let kind = serde_json::from_str(&kind_json).unwrap_or(domain::job::JobKind::Dubbing);

    let status_json: String = row.try_get("status").unwrap_or_default();
    let status = serde_json::from_str(&status_json).unwrap_or(domain::job::JobStatus::Pending);

    let stage_json: Option<String> = row.try_get("stage").ok().flatten();
    let stage = stage_json.and_then(|s| serde_json::from_str(&s).ok());

    let progress_json: String = row.try_get("progress_json").unwrap_or_else(|_| "{}".to_string());
    let progress = serde_json::from_str(&progress_json).unwrap_or_else(|_| domain::job::JobProgress::initializing());

    let error_json: Option<String> = row.try_get("error_json").ok().flatten();
    let error = error_json.and_then(|s| serde_json::from_str(&s).ok());

    let created_at_str: String = row.try_get("created_at").unwrap_or_default();
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let started_at_str: Option<String> = row.try_get("started_at").ok().flatten();
    let started_at = started_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok());

    let finished_at_str: Option<String> = row.try_get("finished_at").ok().flatten();
    let finished_at = finished_at_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&chrono::Utc)).ok());

    Ok(JobSnapshot {
        id,
        project_id,
        kind,
        status,
        stage,
        progress,
        error,
        created_at,
        started_at,
        finished_at,
    })
}
