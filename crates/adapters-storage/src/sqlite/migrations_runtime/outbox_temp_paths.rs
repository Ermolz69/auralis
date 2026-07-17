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
    let canonical_root =
        tokio::fs::canonicalize(workspace_root)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!(
                    "Failed to canonicalize workspace root during migration: {}",
                    e
                ),
            })?;

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, payload_json FROM outbox_messages WHERE kind = 'delete_temp_path'",
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to fetch legacy temp path messages", e)
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
                    match convert_path_to_workspace_key(&p, &canonical_root).await {
                        Ok(workspace_key) => {
                            let new_payload = serde_json::json!({
                                "type": "delete_workspace_file",
                                "workspace_key": workspace_key
                            });
                            let new_payload_str =
                                serde_json::to_string(&new_payload).map_err(|e| {
                                    PortError::Unexpected {
                                        message: format!(
                                            "Failed to serialize new legacy outbox payload: {}",
                                            e
                                        ),
                                    }
                                })?;

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
                        Err(reason) => {
                            sqlx::query(
                                "UPDATE outbox_messages SET status = 'dead', last_error = ? WHERE id = ?"
                            )
                            .bind(reason)
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
    canonical_root: &std::path::Path,
) -> Result<String, &'static str> {
    for comp in path.split(['/', '\\']) {
        if comp == "." || comp == ".." {
            return Err("Legacy DeleteTempPath contains invalid components");
        }
    }

    let target = std::path::PathBuf::from(path);

    let relative = if target.is_absolute() {
        let mut nearest_existing = target.clone();
        let mut missing_components = Vec::new();

        while tokio::fs::metadata(&nearest_existing).await.is_err() {
            if let Some(parent) = nearest_existing.parent() {
                if let Some(name) = nearest_existing.file_name() {
                    missing_components.push(name.to_owned());
                }
                nearest_existing = parent.to_path_buf();
            } else {
                return Err("Path has no existing ancestor");
            }
        }

        missing_components.reverse();

        let canonical_ancestor = tokio::fs::canonicalize(&nearest_existing)
            .await
            .map_err(|_| "Failed to canonicalize nearest ancestor")?;

        let stripped = canonical_ancestor
            .strip_prefix(canonical_root)
            .map_err(|_| "Legacy DeleteTempPath path is outside workspace root")?;

        let mut final_relative = stripped.to_path_buf();
        for comp in missing_components {
            final_relative.push(comp);
        }

        final_relative
    } else {
        target
    };

    let mut relative_str = String::new();
    for comp in relative.components() {
        match comp {
            std::path::Component::Normal(os_str) => {
                if let Some(s) = os_str.to_str() {
                    if !relative_str.is_empty() {
                        relative_str.push('/');
                    }
                    relative_str.push_str(s);
                } else {
                    return Err("Legacy DeleteTempPath contains invalid UTF-8");
                }
            }
            _ => return Err("Legacy DeleteTempPath contains invalid components"),
        }
    }

    if !relative_str.starts_with("tmp/") {
        return Err("Legacy DeleteTempPath does not start with tmp/");
    }

    if domain::outbox::WorkspaceKey::new(relative_str.clone()).is_ok() {
        Ok(relative_str)
    } else {
        Err("Legacy DeleteTempPath is not a valid WorkspaceKey")
    }
}
