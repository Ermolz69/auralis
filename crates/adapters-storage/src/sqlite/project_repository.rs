use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::ProjectRepository;

use super::project_mapper::{project_to_row_values, row_to_project};
use super::project_row::ProjectRow;

pub struct SqliteProjectRepository {
    pool: SqlitePool,
}

impl SqliteProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        let values = project_to_row_values(&project)?;

        sqlx::query(
            r#"
                id, title, status, source_json, metadata_json, 
                source_language, target_language, transcript_json, 
                active_job_id, last_terminal_job_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(values.id)
        .bind(values.title)
        .bind(values.status)
        .bind(values.source_json)
        .bind(values.metadata_json)
        .bind(values.source_language)
        .bind(values.target_language)
        .bind(values.transcript_json)
        .bind(values.active_job_id)
        .bind(values.last_terminal_job_id)
        .bind(values.created_at)
        .bind(values.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to create project: {}", e),
        })?;

        Ok(project)
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT 
                id, title, status, source_json, metadata_json, 
                source_language, target_language, transcript_json, 
                active_job_id, last_terminal_job_id, created_at, updated_at
            FROM projects 
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch project: {}", e),
        })?;

        row.map(row_to_project).transpose()
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        let values = project_to_row_values(project)?;

        let result = sqlx::query(
            r#"
            UPDATE projects SET
                title = ?,
                status = ?,
                source_json = ?,
                metadata_json = ?,
                source_language = ?,
                target_language = ?,
                transcript_json = ?,
                active_job_id = ?,
                last_terminal_job_id = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(values.title)
        .bind(values.status)
        .bind(values.source_json)
        .bind(values.metadata_json)
        .bind(values.source_language)
        .bind(values.target_language)
        .bind(values.transcript_json)
        .bind(values.active_job_id)
        .bind(values.last_terminal_job_id)
        .bind(values.updated_at)
        .bind(values.id)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to update project: {}", e),
        })?;

        if result.rows_affected() == 0 {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }

        Ok(())
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            r#"
            SELECT 
                id, title, status, source_json, metadata_json, 
                source_language, target_language, transcript_json, 
                active_job_id, last_terminal_job_id, created_at, updated_at
            FROM projects 
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list projects: {}", e),
        })?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row_to_project(row)?);
        }

        Ok(projects)
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to delete project: {}", e),
            })?;
        Ok(())
    }
}
