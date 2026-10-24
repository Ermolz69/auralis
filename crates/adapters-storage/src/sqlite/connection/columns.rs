use super::{PortError, SqlitePool, non_final_schema_error};

pub(super) async fn validate_columns(
    pool: &SqlitePool,
    table: &'static str,
    expected: &[&str],
) -> Result<(), PortError> {
    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("Failed to inspect sqlite table", error)
            })?;
    if columns
        .iter()
        .map(String::as_str)
        .eq(expected.iter().copied())
    {
        Ok(())
    } else {
        Err(non_final_schema_error(table))
    }
}
