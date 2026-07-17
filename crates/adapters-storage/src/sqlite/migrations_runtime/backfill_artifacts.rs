use crate::sqlite::helpers;
use domain::media::ArtifactLocation;
use ports::error::PortError;
use sqlx::{Connection, Row, SqlitePool};

#[derive(Debug, serde::Serialize)]
pub struct BackfillIssue {
    pub project_id: String,
    pub artifact_identifier: String,
    pub category: String,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct BackfillReport {
    pub projects_scanned: usize,
    pub projects_migrated: usize,
    pub artifacts_migrated: usize,
    pub artifacts_already_present: usize,
    pub failed_projects: usize,
    pub issues: Vec<BackfillIssue>,
}

/// Runs the legacy artifact backfill process.
/// This reads from the deprecated `artifacts_json` column.
/// See `docs/storage/legacy-artifacts-json.md` for lifecycle and removal details.
pub async fn run(pool: &SqlitePool) -> Result<BackfillReport, PortError> {
    let mut report = BackfillReport::default();

    // Check if runtime_migrations table exists (might be fresh DB)
    let has_runtime_migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_table_info('runtime_migrations')")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    if has_runtime_migrations == 0 {
        // If the table doesn't exist, this is an unexpected state (migrations didn't run).
        // Let it fall through, the query later might fail, but let's check column first.
    } else {
        let marker_exists: bool =
            sqlx::query("SELECT 1 FROM runtime_migrations WHERE id = 'artifacts_json_dropped_v1'")
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
                .is_some();
        if marker_exists {
            return Ok(report);
        }
    }

    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'artifacts_json'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_column == 0 {
        return Ok(report);
    }

    // Read raw JSON from projects table to avoid tying migration to current domain structs
    let rows = sqlx::query("SELECT id, artifacts_json, created_at FROM projects WHERE artifacts_json IS NOT NULL AND artifacts_json != '[]'")
        .fetch_all(pool)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to fetch projects for artifact backfill", e))?;

    for row in rows {
        let project_id: String = match row.try_get("id") {
            Ok(id) => id,
            Err(_) => {
                report.failed_projects += 1;
                continue; // Cannot process without project_id
            }
        };
        let artifacts_json: String = match row.try_get("artifacts_json") {
            Ok(json) => json,
            Err(_) => {
                report.failed_projects += 1;
                continue;
            }
        };
        let project_created_at: String =
            row.try_get("created_at").unwrap_or_else(|_| "".to_string());

        report.projects_scanned += 1;

        let parsed_array: Vec<serde_json::Value> = match serde_json::from_str(&artifacts_json) {
            Ok(a) => a,
            Err(e) => {
                report.issues.push(BackfillIssue {
                    project_id: project_id.clone(),
                    artifact_identifier: "root".to_string(),
                    category: "malformed_json".to_string(),
                    field: "artifacts_json".to_string(),
                    message: format!("Failed to parse JSON array: {}", e),
                });
                report.failed_projects += 1;
                continue;
            }
        };

        #[derive(serde::Deserialize)]
        struct LegacyArtifact {
            id: domain::media::ArtifactId,
            kind: domain::media::ArtifactKind,
            location: domain::media::ArtifactLocation,
            size_bytes: Option<u64>,
            created_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let mut artifacts_to_insert = Vec::new();
        let mut has_errors = false;

        for (idx, value) in parsed_array.into_iter().enumerate() {
            let artifact_id_fallback = value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(&format!("index_{}", idx))
                .to_string();

            let artifact: LegacyArtifact = match serde_json::from_value(value) {
                Ok(a) => a,
                Err(e) => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact_id_fallback,
                        category: "validation_error".to_string(),
                        field: "element".to_string(),
                        message: format!("Failed to parse artifact object: {}", e),
                    });
                    has_errors = true;
                    continue;
                }
            };
            artifacts_to_insert.push(artifact);
        }

        if has_errors {
            report.failed_projects += 1;
            continue;
        }

        let mut tx = pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction for backfill", e)
        })?;

        let mut project_artifacts_migrated = 0;
        let mut project_artifacts_already_present = 0;

        for artifact in artifacts_to_insert {
            let kind = match helpers::serialize_enum(&artifact.kind, "artifact.kind") {
                Ok(k) => k,
                Err(e) => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact.id.to_string(),
                        category: "serialization_error".to_string(),
                        field: "kind".to_string(),
                        message: format!("Failed to serialize kind: {:?}", e),
                    });
                    has_errors = true;
                    break;
                }
            };

            let (location_kind, location_value) = match &artifact.location {
                ArtifactLocation::LocalPath(p) => ("LocalPath".to_string(), p.clone()),
                ArtifactLocation::StorageKey(k) => ("StorageKey".to_string(), k.clone()),
            };

            let safe_size_bytes = match artifact.size_bytes {
                Some(s) if s > i64::MAX as u64 => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact.id.to_string(),
                        category: "validation_error".to_string(),
                        field: "size_bytes".to_string(),
                        message: "size_bytes exceeds i64::MAX".to_string(),
                    });
                    has_errors = true;
                    break;
                }
                Some(s) => Some(s as i64),
                None => None,
            };

            // Fallback to project creation time is an explicit migration policy.
            let created_at = artifact
                .created_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| project_created_at.clone());
            let updated_at = created_at.clone();
            let ready_at = created_at.clone();
            let state = "ready".to_string();

            // Check if existing artifact exists
            let existing_res = sqlx::query_as::<_, (String, String, String, String, Option<i64>)>(
                "SELECT project_id, kind, location_kind, location_value, size_bytes FROM artifacts WHERE id = ?",
            )
            .bind(artifact.id.to_string())
            .fetch_optional(&mut *tx)
            .await;

            let existing = match existing_res {
                Ok(e) => e,
                Err(e) => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact.id.to_string(),
                        category: "database_error".to_string(),
                        field: "select".to_string(),
                        message: format!("Failed to check existing artifact: {}", e),
                    });
                    has_errors = true;
                    break;
                }
            };

            if let Some((ex_proj, ex_kind, ex_loc_kind, ex_loc_val, ex_size)) = existing {
                if ex_proj == project_id
                    && ex_kind == kind
                    && ex_loc_kind == location_kind
                    && ex_loc_val == location_value
                    && ex_size == safe_size_bytes
                {
                    project_artifacts_already_present += 1;
                    continue; // Already migrated
                } else {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact.id.to_string(),
                        category: "conflict".to_string(),
                        field: "id".to_string(),
                        message: "Artifact exists with different data".to_string(),
                    });
                    has_errors = true;
                    break;
                }
            }

            // Check if location is already taken by another ID
            let loc_conflict = sqlx::query_scalar::<_, String>(
                "SELECT id FROM artifacts WHERE location_kind = ? AND location_value = ?",
            )
            .bind(&location_kind)
            .bind(&location_value)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None);

            if loc_conflict.is_some() {
                report.issues.push(BackfillIssue {
                    project_id: project_id.clone(),
                    artifact_identifier: artifact.id.to_string(),
                    category: "conflict".to_string(),
                    field: "location".to_string(),
                    message: "Location is already taken by another artifact".to_string(),
                });
                has_errors = true;
                break;
            }

            if let Err(e) = sqlx::query(
                r#"
                INSERT INTO artifacts (
                    id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(artifact.id.to_string())
            .bind(&project_id)
            .bind(kind)
            .bind(location_kind)
            .bind(location_value)
            .bind(safe_size_bytes)
            .bind(state)
            .bind(created_at)
            .bind(updated_at)
            .bind(ready_at)
            .execute(&mut *tx)
            .await
            {
                report.issues.push(BackfillIssue {
                    project_id: project_id.clone(),
                    artifact_identifier: artifact.id.to_string(),
                    category: "database_error".to_string(),
                    field: "insert".to_string(),
                    message: format!("Failed to insert backfilled artifact: {}", e),
                });
                has_errors = true;
                break;
            }
            project_artifacts_migrated += 1;
        }

        if has_errors {
            report.failed_projects += 1;
            if let Err(e) = tx.rollback().await {
                report.issues.push(BackfillIssue {
                    project_id: project_id.clone(),
                    artifact_identifier: "root".to_string(),
                    category: "database_error".to_string(),
                    field: "rollback".to_string(),
                    message: format!("Failed to rollback transaction: {}", e),
                });
            }
            continue;
        }

        if let Err(e) = tx.commit().await {
            report.issues.push(BackfillIssue {
                project_id: project_id.clone(),
                artifact_identifier: "root".to_string(),
                category: "database_error".to_string(),
                field: "commit".to_string(),
                message: format!("Failed to commit transaction: {}", e),
            });
            report.failed_projects += 1;
            continue;
        }

        report.projects_migrated += 1;
        report.artifacts_migrated += project_artifacts_migrated;
        report.artifacts_already_present += project_artifacts_already_present;
    }

    if report.failed_projects > 0 {
        return Err(PortError::Unexpected {
            message: format!(
                "Backfill failed for {} projects. Check issues for details.",
                report.failed_projects
            ),
        });
    }

    // Phase 3: Table Rebuild & Column Deletion
    // Acquire a single dedicated connection
    let mut conn = pool.acquire().await.map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to acquire connection for rebuild", e)
    })?;

    // Disable foreign keys on this specific connection
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to disable foreign keys", e)
        })?;

    // Perform the rebuild inside a transaction on this connection
    let mut tx = conn.begin().await.map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction for rebuild", e)
    })?;

    let rebuild_result = async {
        sqlx::query(
            r#"
            CREATE TABLE projects_new (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                source_json TEXT,
                metadata_json TEXT,
                source_language TEXT,
                target_language TEXT,
                transcript_json TEXT,
                active_job_id TEXT,
                last_terminal_job_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO projects_new (
                id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, created_at, updated_at
            )
            SELECT 
                id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, created_at, updated_at
            FROM projects
            "#,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("DROP TABLE projects").execute(&mut *tx).await?;
        sqlx::query("ALTER TABLE projects_new RENAME TO projects").execute(&mut *tx).await?;

        sqlx::query("CREATE INDEX idx_projects_updated_at ON projects(updated_at)").execute(&mut *tx).await?;
        sqlx::query("CREATE INDEX idx_projects_status ON projects(status)").execute(&mut *tx).await?;

        let fk_violations = sqlx::query("PRAGMA foreign_key_check").fetch_all(&mut *tx).await?;
        if !fk_violations.is_empty() {
            return Err(sqlx::Error::Protocol("Foreign key check failed after rebuild".to_string()));
        }

        sqlx::query("INSERT INTO runtime_migrations (id, applied_at) VALUES ('artifacts_json_dropped_v1', datetime('now'))")
            .execute(&mut *tx)
            .await?;

        Ok::<(), sqlx::Error>(())
    }
    .await;

    match rebuild_result {
        Ok(_) => {
            tx.commit().await.map_err(|e| {
                crate::sqlite::helpers::map_sqlite_error("Failed to commit rebuild transaction", e)
            })?;
        }
        Err(e) => {
            tx.rollback().await.map_err(|re| PortError::Unexpected {
                message: format!(
                    "Failed to rollback rebuild transaction after error {}: {}",
                    e, re
                ),
            })?;
            // Must re-enable foreign keys before returning connection to pool!
            let _ = sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&mut *conn)
                .await;
            return Err(PortError::Unexpected {
                message: format!("Table rebuild failed: {}", e),
            });
        }
    }

    // Re-enable foreign keys on this specific connection before returning to pool
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to re-enable foreign keys", e)
        })?;

    Ok(report)
}
