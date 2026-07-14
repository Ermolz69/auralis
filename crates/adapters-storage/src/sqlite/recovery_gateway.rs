use async_trait::async_trait;
use sqlx::SqlitePool;
use std::collections::HashSet;

use domain::job::Job;
use domain::project::Project;
use ports::error::PortError;
use ports::recovery::{RecoverySnapshot, RecoveryStorage};

use super::job_mapper::row_to_job;
use super::job_row::JobRow;
use super::project_mapper::row_to_project;
use super::project_row::ProjectRow;

pub struct SqliteRecoveryStorage {
    pool: SqlitePool,
}

impl SqliteRecoveryStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecoveryStorage for SqliteRecoveryStorage {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction for snapshot: {}", e),
        })?;

        // 1. Load all Processing projects
        let project_rows: Vec<ProjectRow> =
            sqlx::query_as("SELECT * FROM projects WHERE status = 'Processing'")
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!("Failed to fetch processing projects: {}", e),
                })?;

        let mut processing_projects = Vec::new();
        let mut linked_job_ids = HashSet::new();

        for row in project_rows {
            if let Some(ref active_id) = row.active_job_id {
                linked_job_ids.insert(active_id.clone());
            }
            processing_projects.push(row_to_project(row)?);
        }

        // 2. Load linked jobs (even if terminal)
        let mut linked_jobs = Vec::new();
        for job_id_str in &linked_job_ids {
            let job_row: Option<JobRow> = sqlx::query_as("SELECT * FROM jobs WHERE id = ?")
                .bind(job_id_str)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!("Failed to fetch linked job {}: {}", job_id_str, e),
                })?;

            if let Some(row) = job_row {
                linked_jobs.push(row_to_job(row)?);
            }
        }

        // 3. Load all other active jobs (Pending/Running) to find orphans
        let active_job_rows: Vec<JobRow> =
            sqlx::query_as("SELECT * FROM jobs WHERE status IN ('Pending', 'Running')")
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!("Failed to fetch active jobs: {}", e),
                })?;

        let mut active_jobs = Vec::new();
        for row in active_job_rows {
            // Only add if not already in linked_jobs
            if !linked_job_ids.contains(&row.id) {
                active_jobs.push(row_to_job(row)?);
            }
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit read transaction: {}", e),
        })?;

        Ok(RecoverySnapshot {
            processing_projects,
            linked_jobs,
            active_jobs,
        })
    }

    async fn commit_interrupted_pair(&self, project: Project, job: Job) -> Result<(), PortError> {
        // Strict conditional update
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin tx: {}", e),
        })?;

        // Expect job to be Pending or Running
        let job_affected = sqlx::query(
            "UPDATE jobs SET status = ?, updated_at = ?, progress_json = ?, error_json = ? 
             WHERE id = ? AND status IN ('Pending', 'Running')",
        )
        .bind(
            serde_json::to_string(job.status())
                .unwrap()
                .trim_matches('"')
                .to_string(),
        )
        .bind(job.updated_at())
        .bind(serde_json::to_string(job.progress()).unwrap())
        .bind(job.error().map(|e| serde_json::to_string(e).unwrap()))
        .bind(job.id().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?
        .rows_affected();

        if job_affected == 0 {
            return Err(PortError::Unexpected {
                message: format!(
                    "Strict update failed for job {}: invariant violated",
                    job.id()
                ),
            });
        }

        // Expect project to be Processing and have active_job_id
        let project_affected = sqlx::query(
            "UPDATE projects SET status = ?, updated_at = ?, active_job_id = ?
             WHERE id = ? AND status = 'Processing' AND active_job_id = ?",
        )
        .bind(
            serde_json::to_string(project.status())
                .unwrap()
                .trim_matches('"')
                .to_string(),
        )
        .bind(project.updated_at())
        .bind(project.active_job_id().map(|id| id.to_string()))
        .bind(project.id().to_string())
        .bind(job.id().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?
        .rows_affected();

        if project_affected == 0 {
            return Err(PortError::Unexpected {
                message: format!(
                    "Strict update failed for project {}: invariant violated",
                    project.id()
                ),
            });
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;
        Ok(())
    }

    async fn commit_reconciled_project(&self, project: Project) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin tx: {}", e),
        })?;

        let status_str = serde_json::to_string(project.status())
            .unwrap()
            .trim_matches('"')
            .to_string();

        let mut q = sqlx::query(
            "UPDATE projects SET status = ?, updated_at = ?, active_job_id = ?
             WHERE id = ? AND status = 'Processing'",
        );
        q = q
            .bind(status_str)
            .bind(project.updated_at())
            .bind(project.active_job_id().map(|id| id.to_string()))
            .bind(project.id().to_string());

        let rows = q
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?
            .rows_affected();

        if rows == 0 {
            return Err(PortError::Unexpected {
                message: format!(
                    "Strict update failed for reconciling project {}",
                    project.id()
                ),
            });
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;
        Ok(())
    }

    async fn commit_failed_project_no_job(&self, project: Project) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin tx: {}", e),
        })?;

        let rows = sqlx::query(
            "UPDATE projects SET status = ?, updated_at = ?, active_job_id = NULL
             WHERE id = ? AND status = 'Processing'",
        )
        .bind(
            serde_json::to_string(project.status())
                .unwrap()
                .trim_matches('"')
                .to_string(),
        )
        .bind(project.updated_at())
        .bind(project.id().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?
        .rows_affected();

        if rows == 0 {
            return Err(PortError::Unexpected {
                message: format!("Strict update failed for project no job {}", project.id()),
            });
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;
        Ok(())
    }

    async fn commit_orphan_job(&self, job: Job) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin tx: {}", e),
        })?;

        let rows = sqlx::query(
            "UPDATE jobs SET status = ?, updated_at = ?, progress_json = ?, error_json = ? 
             WHERE id = ? AND status IN ('Pending', 'Running')",
        )
        .bind(
            serde_json::to_string(job.status())
                .unwrap()
                .trim_matches('"')
                .to_string(),
        )
        .bind(job.updated_at())
        .bind(serde_json::to_string(job.progress()).unwrap())
        .bind(job.error().map(|e| serde_json::to_string(e).unwrap()))
        .bind(job.id().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?
        .rows_affected();

        if rows == 0 {
            return Err(PortError::Unexpected {
                message: format!("Strict update failed for orphan job {}", job.id()),
            });
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;
        Ok(())
    }
}
