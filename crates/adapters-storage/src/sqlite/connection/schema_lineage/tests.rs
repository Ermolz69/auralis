use super::super::{audio_assembly, puza_project, speech_limits, speech_workflow};
use crate::sqlite::{
    connect_sqlite,
    connection::{SCHEMA, create_pool},
};
use sqlx::SqlitePool;
use std::error::Error;

async fn base(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    sqlx::raw_sql(SCHEMA).execute(pool).await?;
    crate::sqlite::outbox_parallel_migration::migrate(pool).await?;
    speech_workflow::migrate_and_validate(pool, 13).await?;
    puza_project::migrate_and_validate(pool, 16).await?;
    Ok(())
}

#[tokio::test]
async fn legacy_media_branch_preserves_library_and_expands_speech_limits()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("legacy-media.sqlite");
    let pool = create_pool(&path).await?;
    base(&pool).await?;
    sqlx::raw_sql(include_str!("../../library_schema.sql"))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!("../../media_project_schema.sql"))
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO workspace_library VALUES(1,4,'{\"assets\":[]}')")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version = 19")
        .execute(&pool)
        .await?;
    pool.close().await;
    let pool = connect_sqlite(&path).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(version, 22);
    let payload: String = sqlx::query_scalar("SELECT payload FROM workspace_library WHERE id=1")
        .fetch_one(&pool)
        .await?;
    assert_eq!(payload, "{\"assets\":[]}");
    let schema: String =
        sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE name='speech_run_intents'")
            .fetch_one(&pool)
            .await?;
    assert!(schema.contains("8192"));
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_optional(&pool)
            .await?
            .is_none()
    );
    pool.close().await;
    let reopened = connect_sqlite(&path).await?;
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn canonical_audio_schema_twenty_reaches_the_combined_schema() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("audio.sqlite");
    let pool = create_pool(&path).await?;
    base(&pool).await?;
    speech_limits::migrate(&pool, 17).await?;
    audio_assembly::migrate_and_validate(&pool, 18).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(version, 20);
    pool.close().await;
    let pool = connect_sqlite(&path).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(version, 22);
    assert!(
        sqlx::query("SELECT * FROM workspace_library")
            .fetch_all(&pool)
            .await?
            .is_empty()
    );
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn partial_media_branch_is_refused_without_promoting_its_version()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("partial.sqlite");
    let pool = create_pool(&path).await?;
    base(&pool).await?;
    sqlx::raw_sql(include_str!("../../library_schema.sql"))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!("../../media_project_schema.sql"))
        .execute(&pool)
        .await?;
    sqlx::query("DROP TABLE media_run_sources")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA user_version = 19")
        .execute(&pool)
        .await?;
    assert!(connect_sqlite(&path).await.is_err());
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await?;
    assert_eq!(version, 19);
    pool.close().await;
    Ok(())
}
