use ports::error::PortError;
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LegacyOutboxPayload {
    DeleteTempPath {
        path: Option<String>,
        absolute_path: Option<String>,
    },
    #[serde(other)]
    Other,
}

pub async fn migrate_delete_temp_paths(
    tx: &mut Transaction<'_, Sqlite>,
    workspace_root: &std::path::Path,
) -> Result<(), PortError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, payload_json FROM outbox_messages WHERE kind = 'delete_temp_path'",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to fetch legacy temp path messages: {}", e),
    })?;

    for (id, payload_json) in rows {
        let legacy: Result<LegacyOutboxPayload, _> = serde_json::from_str(&payload_json);
        match legacy {
            Ok(LegacyOutboxPayload::DeleteTempPath {
                path,
                absolute_path,
            }) => {
                let p = path.or(absolute_path);
                if let Some(p) = p {
                    match convert_path_to_workspace_key(&p, workspace_root).await {
                        Some(workspace_key) => {
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
                            sqlx::query(
                                "UPDATE outbox_messages SET status = 'dead', last_error = ? WHERE id = ?"
                            )
                            .bind("Legacy DeleteTempPath path is outside workspace root")
                            .bind(&id)
                            .execute(&mut **tx)
                            .await
                            .map_err(|e| PortError::Unexpected {
                                message: format!("Failed to mark legacy temp path message dead {}: {}", id, e),
                            })?;
                        }
                    }
                } else {
                    sqlx::query(
                        "UPDATE outbox_messages SET status = 'dead', last_error = ? WHERE id = ?",
                    )
                    .bind("Legacy DeleteTempPath had no valid path fields")
                    .bind(&id)
                    .execute(&mut **tx)
                    .await
                    .map_err(|e| PortError::Unexpected {
                        message: format!(
                            "Failed to mark empty temp path message dead {}: {}",
                            id, e
                        ),
                    })?;
                }
            }
            _ => {
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

async fn convert_path_to_workspace_key(
    path: &str,
    workspace_root: &std::path::Path,
) -> Option<String> {
    let target = std::path::PathBuf::from(path);

    // If it's absolute, check if it's within workspace root
    if target.is_absolute() {
        let canonical_root = tokio::fs::canonicalize(workspace_root)
            .await
            .unwrap_or_else(|_| workspace_root.to_path_buf());

        // We do a string prefix check or strip_prefix
        // Strip prefix works if they are normalized. We can just use `strip_prefix` on the raw path
        // since `canonicalize` might fail if `target` doesn't exist yet, which is the case for temp files.
        let target_normalized = path.replace('\\', "/");
        let root_str = canonical_root.to_string_lossy().replace('\\', "/");

        let relative_str = if target_normalized.starts_with(&root_str) {
            target_normalized[root_str.len()..]
                .trim_start_matches('/')
                .to_string()
        } else {
            return None;
        };

        if domain::outbox::WorkspaceKey::new(relative_str.clone()).is_ok() {
            Some(relative_str)
        } else {
            None
        }
    } else {
        let path_normalized = path.replace('\\', "/");
        if domain::outbox::WorkspaceKey::new(path_normalized.clone()).is_ok() {
            Some(path_normalized)
        } else {
            None
        }
    }
}
