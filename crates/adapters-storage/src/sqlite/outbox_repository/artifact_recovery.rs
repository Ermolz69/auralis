use super::SqliteOutboxRepository;
use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{artifact_recovery::ArtifactRecoveryRepository, error::PortError};

#[async_trait]
impl ArtifactRecoveryRepository for SqliteOutboxRepository {
    async fn list_projects(&self) -> Result<Vec<ProjectId>, PortError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT a.project_id FROM artifacts a JOIN projects p ON p.id = a.project_id
             JOIN outbox_messages o ON a.id = json_extract(CASE WHEN json_valid(o.payload_json) THEN o.payload_json ELSE '{}' END, '$.artifact_id')
             WHERE o.kind = 'finalize_staged_artifact' AND o.status = 'dead' AND a.state = 'pending_finalize'
             ORDER BY a.project_id"
        ).fetch_all(&self.pool).await.map_err(map_error)?;
        ids.into_iter()
            .map(|id| {
                id.parse().map_err(|_| PortError::InvalidStoredData {
                    entity_type: "ArtifactRecovery".into(),
                    entity_id: id,
                    field: "project_id".into(),
                    message: "Invalid project identifier".into(),
                })
            })
            .collect()
    }

    async fn retry_finalization(&self, project_id: &ProjectId) -> Result<(), PortError> {
        sqlx::query(
            "UPDATE outbox_messages SET status = 'pending', attempts = 0, last_error = NULL,
                locked_at = NULL, locked_by = NULL,
                next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE kind = 'finalize_staged_artifact' AND status = 'dead'
             AND EXISTS (
                SELECT 1 FROM artifacts a JOIN projects p ON p.id = a.project_id
                WHERE a.project_id = ? AND a.state = 'pending_finalize'
                AND a.id = json_extract(CASE WHEN json_valid(payload_json) THEN payload_json ELSE '{}' END, '$.artifact_id')
             )"
        ).bind(project_id.to_string()).execute(&self.pool).await.map_err(map_error)?;
        Ok(())
    }
}

fn map_error(error: sqlx::Error) -> PortError {
    crate::sqlite::helpers::map_sqlite_error("artifact_recovery", error)
}
