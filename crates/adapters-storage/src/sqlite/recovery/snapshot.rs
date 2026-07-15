use sqlx::SqlitePool;
use std::collections::HashSet;

use ports::error::PortError;
use ports::recovery::RecoverySnapshot;

use super::super::job_mapper::row_to_job;
use super::super::job_row::JobRow;
use super::super::project_mapper::row_to_project;
use super::super::project_row::ProjectRow;

pub async fn load_snapshot(pool: &SqlitePool) -> Result<RecoverySnapshot, PortError> {
    let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
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

    // 3. Load all other active jobs (Pending/Running) to find orphans and multiple active jobs
    let active_job_rows: Vec<JobRow> =
        sqlx::query_as("SELECT * FROM jobs WHERE status IN ('Pending', 'Running')")
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to fetch active jobs: {}", e),
            })?;

    let mut active_jobs = Vec::new();
    for row in active_job_rows {
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
