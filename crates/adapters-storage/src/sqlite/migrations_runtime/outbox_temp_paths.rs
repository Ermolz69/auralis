use ports::error::PortError;
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LegacyOutboxPayload {
    DeleteTempPath {
        path: String,
    },
    #[serde(other)]
    Other,
}

pub async fn migrate_delete_temp_paths(tx: &mut Transaction<'_, Sqlite>) -> Result<(), PortError> {
    // We only fetch rows that are `delete_temp_path`
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, payload_json FROM outbox_messages WHERE kind = 'delete_temp_path'",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to fetch legacy temp path messages: {}", e),
    })?;

    for (id, payload_json) in rows {
        // Parse legacy
        let legacy: Result<LegacyOutboxPayload, _> = serde_json::from_str(&payload_json);
        match legacy {
            Ok(LegacyOutboxPayload::DeleteTempPath { path }) => {
                // Determine if we can convert it to DeleteWorkspaceFile
                match convert_path_to_workspace_key(&path) {
                    Some(workspace_key) => {
                        // We can convert!
                        let new_payload = serde_json::json!({
                            "type": "delete_workspace_file",
                            "workspace_key": workspace_key
                        });
                        let new_payload_str = serde_json::to_string(&new_payload).unwrap();

                        sqlx::query(
                            "UPDATE outbox_messages SET kind = 'delete_workspace_file', payload_json = ? WHERE id = ?"
                        )
                        .bind(new_payload_str)
                        .bind(&id)
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| PortError::Unexpected {
                            message: format!("Failed to update legacy temp path message {}: {}", id, e),
                        })?;
                    }
                    None => {
                        // Cannot safely convert. Mark as dead.
                        sqlx::query(
                            "UPDATE outbox_messages SET status = 'dead', last_error = ? WHERE id = ?"
                        )
                        .bind("Legacy DeleteTempPath could not be converted to WorkspaceKey")
                        .bind(&id)
                        .execute(&mut **tx)
                        .await
                        .map_err(|e| PortError::Unexpected {
                            message: format!("Failed to mark legacy temp path message dead {}: {}", id, e),
                        })?;
                    }
                }
            }
            _ => {
                // Maybe it's malformed? Mark as dead.
                sqlx::query(
                    "UPDATE outbox_messages SET status = 'dead', last_error = ? WHERE id = ?",
                )
                .bind("Legacy DeleteTempPath payload parsing failed")
                .bind(&id)
                .execute(&mut **tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!(
                        "Failed to mark malformed temp path message dead {}: {}",
                        id, e
                    ),
                })?;
            }
        }
    }

    Ok(())
}

fn convert_path_to_workspace_key(path: &str) -> Option<String> {
    // If it's absolute, we can't safely convert. We don't know the workspace root.
    if path.starts_with('/') || path.starts_with('\\') || path.chars().nth(1) == Some(':') {
        return None;
    }

    // Convert all `\` to `/`
    let path = path.replace('\\', "/");

    // Check if it's a valid WorkspaceKey
    if domain::outbox::WorkspaceKey::new(path.clone()).is_ok() {
        Some(path)
    } else {
        None
    }
}
