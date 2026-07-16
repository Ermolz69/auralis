use crate::sqlite::preflight::error::DatabaseTransitionError;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::path::Path;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum LegacySchemaVersion {
    HandcraftedV0, // For now, only one version
}

#[derive(Debug, PartialEq, Eq)]
pub enum DatabaseClassification {
    Missing,
    Empty,
    ManagedSqlx,
    KnownLegacy(LegacySchemaVersion),
    Unknown,
    Corrupt,
}

pub struct LegacyDatabaseInspector;

impl LegacyDatabaseInspector {
    /// Opens the database in a strictly read-only mode without mutating the file or its pragmas.
    async fn open_readonly(db_path: &Path) -> Result<SqlitePool, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(false)
            .read_only(true);
        // Note: No journal_mode or foreign_keys Pragmas

        sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
    }

    pub async fn inspect(
        db_path: &Path,
    ) -> Result<DatabaseClassification, DatabaseTransitionError> {
        if !db_path.exists() {
            return Ok(DatabaseClassification::Missing);
        }

        let pool = match Self::open_readonly(db_path).await {
            Ok(pool) => pool,
            Err(e) => {
                if let sqlx::Error::Database(ref db_err) = e {
                    let code = db_err.code().unwrap_or_default();
                    if code == "11" || code == "26" {
                        // 11 = SQLITE_CORRUPT, 26 = SQLITE_NOTADB
                        return Ok(DatabaseClassification::Corrupt);
                    }
                }
                return Err(DatabaseTransitionError::InspectionFailed(e.to_string()));
            }
        };

        // Check tables
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

        if tables.is_empty() {
            pool.close().await;
            return Ok(DatabaseClassification::Empty);
        }

        if tables.contains(&"_sqlx_migrations".to_string()) {
            pool.close().await;
            return Ok(DatabaseClassification::ManagedSqlx);
        }

        // Fingerprint checks for HandcraftedV0
        if Self::check_handcrafted_v0(&pool, &tables).await? {
            pool.close().await;
            return Ok(DatabaseClassification::KnownLegacy(
                LegacySchemaVersion::HandcraftedV0,
            ));
        }

        pool.close().await;
        Ok(DatabaseClassification::Unknown)
    }

    async fn check_handcrafted_v0(
        pool: &SqlitePool,
        tables: &[String],
    ) -> Result<bool, DatabaseTransitionError> {
        // Must contain exactly `projects` (and maybe `artifacts` if it's considered part of V0, but the user explicitly requested checking exact match. We'll check for `projects`).
        // Actually, let's verify what `projects` has.
        if !tables.contains(&"projects".to_string()) {
            return Ok(false);
        }

        // We require no other tables for V0, or maybe we know which tables existed.
        // Previously, `has_projects` was enough for `KnownLegacy`. But now we need a strict fingerprint.
        // Let's assume V0 only had `projects`.
        if tables.len() != 1 {
            return Ok(false);
        }

        // Check columns of projects
        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct ColumnInfo {
            name: String,
            r#type: String,
            notnull: i64,
            pk: i64,
        }

        let columns: Vec<ColumnInfo> = sqlx::query_as("PRAGMA table_info('projects')")
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

        // The expected V0 columns for auralis handcrafted dev DB
        let expected_cols = [
            "id",
            "title",
            "status",
            "artifacts_json",
            "created_at",
            "updated_at",
        ];

        if columns.len() != expected_cols.len() {
            return Ok(false);
        }

        for (col, expected) in columns.iter().zip(expected_cols.iter()) {
            if col.name != *expected {
                return Ok(false);
            }
        }

        // Check primary key is `id`
        if let Some(id_col) = columns.iter().find(|c| c.name == "id") {
            if id_col.pk == 0 {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        // Check foreign keys
        let fks: Vec<String> = sqlx::query_scalar("PRAGMA foreign_key_list('projects')")
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseTransitionError::InspectionFailed(e.to_string()))?;

        if !fks.is_empty() {
            return Ok(false); // Handcrafted V0 projects had no foreign keys
        }

        Ok(true)
    }
}
