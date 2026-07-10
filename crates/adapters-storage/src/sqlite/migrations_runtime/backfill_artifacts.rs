use domain::media::ArtifactLocation;
use ports::error::PortError;
use sqlx::{Row, SqlitePool};

pub async fn run(pool: &SqlitePool) -> Result<(), PortError> {
    // Read raw JSON from projects table to avoid tying migration to current domain structs
    let rows = sqlx::query("SELECT id, artifacts_json FROM projects WHERE artifacts_json IS NOT NULL AND artifacts_json != '[]'")
        .fetch_all(pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch projects for artifact backfill: {}", e),
        })?;

    for row in rows {
        let project_id: String = row.get("id");
        let artifacts_json: String = row.get("artifacts_json");

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

        for artifact in artifacts {
            let kind = serde_json::to_string(&artifact.kind)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();

            let (location_kind, location_value) = match &artifact.location {
                ArtifactLocation::LocalPath(p) => ("LocalPath".to_string(), p.clone()),
                ArtifactLocation::StorageKey(k) => ("StorageKey".to_string(), k.clone()),
            };

            let created_at = artifact
                .created_at
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();
            let updated_at = created_at.clone();
            let ready_at = created_at.clone();
            let state = "ready".to_string();

            sqlx::query(
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
            .execute(pool)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to insert backfilled artifact: {}", e),
            })?;
        }

        // Clear the artifacts_json now that they are migrated
        sqlx::query("UPDATE projects SET artifacts_json = '[]' WHERE id = ?")
            .bind(&project_id)
            .execute(pool)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!(
                    "Failed to clear artifacts_json for project {}: {}",
                    project_id, e
                ),
            })?;
    }

    Ok(())
}
