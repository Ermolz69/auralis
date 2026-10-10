use ports::error::PortError;
use ports::repository::OutboxProtectedResources;

use super::SqliteOutboxRepository;

impl SqliteOutboxRepository {
    pub(super) async fn execute_protected_resources(
        &self,
    ) -> Result<OutboxProtectedResources, PortError> {
        let staging_keys: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT json_extract(payload_json, '$.staging_key')
             FROM outbox_messages WHERE kind = 'finalize_staged_artifact'
               AND status != 'done' AND json_valid(payload_json)
               AND json_type(payload_json, '$.staging_key') = 'text'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("protect_outbox_staging", error)
        })?;
        let raw_workspaces: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT json_extract(payload_json, '$.workspace_key')
             FROM outbox_messages WHERE kind = 'delete_workspace_allocation'
               AND status != 'done' AND json_valid(payload_json)
               AND json_type(payload_json, '$.workspace_key') = 'text'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|error| {
            crate::sqlite::helpers::map_sqlite_error("protect_outbox_workspace", error)
        })?;
        let workspace_keys = raw_workspaces
            .into_iter()
            .map(|key| {
                domain::outbox::WorkspaceKey::new(key.clone()).map_err(|error| {
                    PortError::InvalidStoredData {
                        entity_type: "outbox".into(),
                        entity_id: key,
                        field: "workspace_key".into(),
                        message: error,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(OutboxProtectedResources {
            staging_keys,
            workspace_keys,
        })
    }
}
