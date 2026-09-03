use super::{
    SqliteProjectRepository, connect_sqlite,
    connection::{SCHEMA, create_pool},
};
use ports::repository::ProjectRepository;

#[tokio::test]
async fn version_one_upgrade_preserves_project_data_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("version-one.sqlite");
    let pool = create_pool(&path).await.unwrap();
    let old_schema = SCHEMA.replace("\r\n", "\n").replace("PRAGMA user_version = 2;", "PRAGMA user_version = 1;")
        .replace(",\n    revision INTEGER NOT NULL DEFAULT 1\n        CHECK (typeof(revision) = 'integer' AND revision >= 1)", "");
    sqlx::raw_sql(&old_schema).execute(&pool).await.unwrap();
    let project = domain::project::Project::new("Preserved title".into());
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, 'Draft', ?, ?)")
        .bind(project.id().to_string()).bind(project.title()).bind(project.created_at().to_rfc3339()).bind(project.updated_at().to_rfc3339())
        .execute(&pool).await.unwrap();
    pool.close().await;

    let pool = connect_sqlite(&path).await.unwrap();
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 2);
    let repo = SqliteProjectRepository::new(pool.clone());
    assert_eq!(repo.get(project.id()).await.unwrap().unwrap(), project);
    let renamed = repo
        .update(
            project.id(),
            1,
            ports::project_update::ProjectUpdate::Rename {
                title: "New title".into(),
            },
            project.updated_at(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.revision(), 2);
    pool.close().await;

    let pool = connect_sqlite(&path).await.unwrap();
    assert_eq!(
        SqliteProjectRepository::new(pool)
            .get(project.id())
            .await
            .unwrap()
            .unwrap(),
        renamed
    );
}

#[tokio::test]
async fn unsupported_version_one_schema_is_not_partially_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unsupported.sqlite");
    let pool = create_pool(&path).await.unwrap();
    sqlx::raw_sql("PRAGMA user_version = 1; CREATE TABLE projects (id TEXT);")
        .execute(&pool)
        .await
        .unwrap();
    assert!(connect_sqlite(&path).await.is_err());
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 1);
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('projects')")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(columns, vec!["id"]);
}
