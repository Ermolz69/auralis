use crate::sqlite::helpers;
use domain::media::ArtifactLocation;
use ports::error::PortError;
use sqlx::{Row, SqlitePool};

/// Runs the legacy artifact backfill process.
/// This reads from the deprecated `artifacts_json` column.
/// See `docs/storage/legacy-artifacts-json.md` for lifecycle and removal details.
pub async fn run(pool: &SqlitePool) -> Result<(), PortError> {
    // Read raw JSON from projects table to avoid tying migration to current domain structs
    let rows = sqlx::query("SELECT id, artifacts_json, created_at FROM projects WHERE artifacts_json IS NOT NULL AND artifacts_json != '[]'")
        .fetch_all(pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch projects for artifact backfill: {}", e),
        })?;

    for row in rows {
        let project_id: String = row.get("id");
        let artifacts_json: String = row.get("artifacts_json");
        let project_created_at: String = row.get("created_at");

        #[derive(serde::Deserialize)]
        struct LegacyArtifact {
            id: domain::media::ArtifactId,
            kind: domain::media::ArtifactKind,
            location: domain::media::ArtifactLocation,
            size_bytes: Option<u64>,
            created_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let artifacts: Vec<LegacyArtifact> = match serde_json::from_str(&artifacts_json) {
            Ok(a) => a,
            Err(e) => {
                println!(
                    "WARNING: Failed to parse artifacts_json for project {}: {}",
                    project_id, e
                );
                continue;
            }
        };

        let mut tx = pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction for backfill: {}", e),
        })?;

        let mut all_inserted = true;

        for artifact in artifacts {
            let kind = match helpers::serialize_enum(&artifact.kind, "artifact.kind") {
                Ok(k) => k,
                Err(e) => {
                    println!(
                        "WARNING: Failed to serialize kind for artifact {}: {:?}",
                        artifact.id, e
                    );
                    all_inserted = false;
                    break;
                }
            };

            let (location_kind, location_value) = match &artifact.location {
                ArtifactLocation::LocalPath(p) => ("LocalPath".to_string(), p.clone()),
                ArtifactLocation::StorageKey(k) => ("StorageKey".to_string(), k.clone()),
            };

            let created_at = artifact
                .created_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| project_created_at.clone());
            let updated_at = created_at.clone();
            let ready_at = created_at.clone();
            let state = "ready".to_string();

            if let Err(e) = sqlx::query(
                r#"
                INSERT OR IGNORE INTO artifacts (
                    id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(artifact.id.to_string())
            .bind(&project_id)
            .bind(kind)
            .bind(location_kind)
            .bind(location_value)
            .bind(artifact.size_bytes.map(|s| s as i64))
            .bind(state)
            .bind(created_at)
            .bind(updated_at)
            .bind(ready_at)
            .execute(&mut *tx)
            .await
            {
                println!("WARNING: Failed to insert backfilled artifact {}: {}", artifact.id, e);
                all_inserted = false;
                break;
            }
        }

        if all_inserted {
            // Clear the artifacts_json now that they are migrated
            if let Err(e) = sqlx::query("UPDATE projects SET artifacts_json = '[]' WHERE id = ?")
                .bind(&project_id)
                .execute(&mut *tx)
                .await
            {
                println!(
                    "WARNING: Failed to clear artifacts_json for project {}: {}",
                    project_id, e
                );
                let _ = tx.rollback().await;
            } else {
                let _ = tx.commit().await;
            }
        } else {
            let _ = tx.rollback().await;
        }
    }

    Ok(())
}
