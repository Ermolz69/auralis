use super::{PortError, SqlitePool};
use crate::sqlite::helpers::map_sqlite_error;
use sqlx::{Connection, SqliteConnection};

pub(super) async fn migrate(pool: &SqlitePool, marker: i64) -> Result<(), PortError> {
    if marker >= 18 {
        return Ok(());
    }
    // Rebuilding the parent with FK enforcement enabled would cascade-delete
    // attempts/reviews/publications. Use a detached, dedicated connection:
    // foreign_keys is connection-local and this handle never returns to the pool.
    let mut connection = pool.acquire().await.map_err(error)?.detach();
    let outcome = rebuild(&mut connection).await;
    let closed = connection.close().await.map_err(error);
    outcome?;
    closed
}

async fn rebuild(connection: &mut SqliteConnection) -> Result<(), PortError> {
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *connection)
        .await
        .map_err(error)?;
    let mut tx = connection
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(error)?;
    sqlx::raw_sql(include_str!("../speech_limits_schema.sql"))
        .execute(&mut *tx)
        .await
        .map_err(error)?;
    let broken = sqlx::query("PRAGMA foreign_key_check")
        .fetch_optional(&mut *tx)
        .await
        .map_err(error)?;
    if broken.is_some() {
        return Err(PortError::Storage {
            operation: "migrate_speech_limits",
            message: "Foreign-key validation failed; speech-limit migration rolled back".into(),
        });
    }
    tx.commit().await.map_err(error)
}

fn error(error: sqlx::Error) -> PortError {
    map_sqlite_error("migrate_speech_limits", error)
}

#[cfg(test)]
#[path = "../speech_limits_tests.rs"]
mod tests;
