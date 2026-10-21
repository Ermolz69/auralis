use super::{PortError, SqlitePool, validate_columns};

pub(super) fn extend_expected_tables(tables: &mut Vec<&'static str>, marker: i64) {
    if marker >= 15 {
        tables.extend([
            "speech_text_reviews",
            "speech_publications",
            "speech_run_reviews",
        ]);
    }
    if marker >= 16 {
        tables.push("speech_discarded_artifacts");
    }
    if marker >= 17 {
        tables.push("puza_contract_projects");
    }
}

pub(super) async fn migrate_and_validate(pool: &SqlitePool, marker: i64) -> Result<(), PortError> {
    if marker < 17 {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_puza_project", e))?;
        sqlx::raw_sql(include_str!("../puza_project_schema.sql"))
            .execute(&mut *tx)
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_puza_project", e))?;
        tx.commit()
            .await
            .map_err(|e| crate::sqlite::helpers::map_sqlite_error("migrate_puza_project", e))?;
    }
    validate_columns(
        pool,
        "puza_contract_projects",
        &["project_id", "snapshot_json", "snapshot_sha256"],
    )
    .await
}
