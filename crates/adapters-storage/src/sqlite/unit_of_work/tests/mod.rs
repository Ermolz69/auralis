#![allow(clippy::unwrap_used)]

use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

mod delete;
mod finalize;
mod import;
mod import_atomicity;
mod project_revision;

pub(super) async fn setup_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::raw_sql(crate::sqlite::connection::SCHEMA)
        .execute(&pool)
        .await
        .unwrap();
    crate::sqlite::outbox_parallel_migration::migrate(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(concat!(
        include_str!("../../speech_workflow_schema.sql"),
        include_str!("../../speech_discard_schema.sql"),
        include_str!("../../speech_limits_schema.sql"),
        include_str!("../../audio_assembly_schema.sql")
    ))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();

    pool
}
