use super::repository::{conflict, error};
use ports::{error::PortError, ui_preferences::StoredTheme};
use sqlx::SqlitePool;

pub(super) async fn get(pool: &SqlitePool) -> Result<Option<StoredTheme>, PortError> {
    let row: Option<(String, i64)> =
        sqlx::query_as("SELECT value, revision FROM ui_settings WHERE key = 'color_theme'")
            .fetch_optional(pool)
            .await
            .map_err(error)?;
    Ok(row.map(|(value, revision)| StoredTheme {
        value,
        revision: revision as u64,
    }))
}
pub(super) async fn set(
    pool: &SqlitePool,
    value: &str,
    expected: u64,
) -> Result<StoredTheme, PortError> {
    let row: Option<(String, i64)> = if expected == 0 {
        sqlx::query_as("INSERT INTO ui_settings(key, value, revision) VALUES ('color_theme', ?, 1) ON CONFLICT DO NOTHING RETURNING value, revision")
            .bind(value).fetch_optional(pool).await.map_err(error)?
    } else {
        sqlx::query_as("UPDATE ui_settings SET value = ?, revision = revision + 1 WHERE key = 'color_theme' AND revision = ? RETURNING value, revision")
            .bind(value).bind(i64::try_from(expected).unwrap_or(-1)).fetch_optional(pool).await.map_err(error)?
    };
    row.map(|(value, revision)| StoredTheme {
        value,
        revision: revision as u64,
    })
    .ok_or_else(conflict)
}
pub(super) async fn import(pool: &SqlitePool, value: &str) -> Result<StoredTheme, PortError> {
    sqlx::query("INSERT INTO ui_settings(key, value, revision) VALUES ('color_theme', ?, 1) ON CONFLICT DO NOTHING")
        .bind(value).execute(pool).await.map_err(error)?;
    get(pool).await?.ok_or_else(conflict)
}
