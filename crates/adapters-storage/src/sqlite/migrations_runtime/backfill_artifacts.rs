use crate::sqlite::helpers;
use domain::media::ArtifactLocation;
use ports::error::PortError;
use sqlx::{Row, SqlitePool};

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

    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'artifacts_json'",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to check artifacts_json column", e)
    })?;

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
            let loc_conflict_res = sqlx::query_scalar::<_, String>(
                "SELECT id FROM artifacts WHERE location_kind = ? AND location_value = ?",
            )
            .bind(&location_kind)
            .bind(&location_value)
            .fetch_optional(&mut *tx)
            .await;

            let loc_conflict = match loc_conflict_res {
                Ok(opt) => opt,
                Err(e) => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: artifact.id.to_string(),
                        category: "database_error".to_string(),
                        field: "location_check".to_string(),
                        message: format!("Failed to check location conflict: {}", e),
                    });
                    has_errors = true;
                    break;
                }
            };

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

        if !has_errors {
            let update_res = sqlx::query(
                "UPDATE projects SET artifacts_json = '[]' WHERE id = ? AND artifacts_json = ?",
            )
            .bind(&project_id)
            .bind(&artifacts_json)
            .execute(&mut *tx)
            .await;

            match update_res {
                Ok(result) => {
                    if result.rows_affected() != 1 {
                        report.issues.push(BackfillIssue {
                            project_id: project_id.clone(),
                            artifact_identifier: "root".to_string(),
                            category: "conflict".to_string(),
                            field: "artifacts_json".to_string(),
                            message: "Optimistic lock failed: project was modified or deleted"
                                .to_string(),
                        });
                        has_errors = true;
                    }
                }
                Err(e) => {
                    report.issues.push(BackfillIssue {
                        project_id: project_id.clone(),
                        artifact_identifier: "root".to_string(),
                        category: "database_error".to_string(),
                        field: "clear_json".to_string(),
                        message: format!("Failed to clear legacy JSON: {}", e),
                    });
                    has_errors = true;
                }
            }
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

    Ok(report)
}
