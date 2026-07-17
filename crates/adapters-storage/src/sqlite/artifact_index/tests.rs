#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::repository::SqliteArtifactIndex;
use super::test_support::{make_artifact, setup_db};
use crate::sqlite::{SqliteProjectRepository, connect_sqlite};
use domain::media::{ArtifactKind, ArtifactLocation};
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use tempfile::tempdir;

#[tokio::test]
async fn test_add_inserts_artifact_and_get_returns_same() {
    let (_pool, index, _repo, project) = setup_db().await;
    let artifact = make_artifact(ArtifactKind::LogFile, "1");

    // 1. add inserts artifact
    index.add(project.id(), &artifact).await.unwrap();

    // 2. get returns same artifact
    let fetched = index.get(&artifact.id).await.unwrap().unwrap();
    assert_eq!(fetched.id, artifact.id);
    assert_eq!(fetched.kind, artifact.kind);
    assert_eq!(fetched.size_bytes, artifact.size_bytes);
    if let ArtifactLocation::StorageKey(key) = fetched.location {
        assert_eq!(key, "test-key-1.txt");
    } else {
        panic!("Expected StorageKey");
    }
}

#[tokio::test]
async fn test_list_by_project_returns_only_project_artifacts() {
    let (_pool, index, repo, project1) = setup_db().await;
    let project2 = Project::new("Test Project 2".to_string());
    repo.create(project2.clone()).await.unwrap();

    let artifact1 = make_artifact(ArtifactKind::LogFile, "p1");
    let artifact2 = make_artifact(ArtifactKind::GeneratedTranscript, "p2");

    index.add(project1.id(), &artifact1).await.unwrap();
    index.add(project2.id(), &artifact2).await.unwrap();

    // 3. list_by_project returns only project artifacts
    let p1_artifacts = index.list_by_project(project1.id()).await.unwrap();
    assert_eq!(p1_artifacts.len(), 1);
    assert_eq!(p1_artifacts[0].id, artifact1.id);
}

#[tokio::test]
async fn test_list_by_project_and_kind_filters_correctly() {
    let (_pool, index, _repo, project) = setup_db().await;

    let a_log1 = make_artifact(ArtifactKind::LogFile, "l1");
    let a_log2 = make_artifact(ArtifactKind::LogFile, "l2");
    let a_json = make_artifact(ArtifactKind::GeneratedTranscript, "j1");

    index.add(project.id(), &a_log1).await.unwrap();
    index.add(project.id(), &a_log2).await.unwrap();
    index.add(project.id(), &a_json).await.unwrap();

    // 4. list_by_project_and_kind filters correctly
    let logs = index
        .list_by_project_and_kind(project.id(), ArtifactKind::LogFile)
        .await
        .unwrap();
    assert_eq!(logs.len(), 2);

    let jsons = index
        .list_by_project_and_kind(project.id(), ArtifactKind::GeneratedTranscript)
        .await
        .unwrap();
    assert_eq!(jsons.len(), 1);
    assert_eq!(jsons[0].id, a_json.id);
}

#[tokio::test]
async fn test_delete_removes_artifact_row() {
    let (_pool, index, _repo, project) = setup_db().await;
    let artifact = make_artifact(ArtifactKind::LogFile, "del");

    index.add(project.id(), &artifact).await.unwrap();
    assert!(index.get(&artifact.id).await.unwrap().is_some());

    // 5. delete removes artifact row
    index.delete(&artifact.id).await.unwrap();
    assert!(index.get(&artifact.id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_deleting_project_cascades_artifact_rows() {
    let (_pool, index, repo, project) = setup_db().await;
    let artifact = make_artifact(ArtifactKind::LogFile, "casc");
    index.add(project.id(), &artifact).await.unwrap();

    // 6. deleting project cascades artifact rows
    repo.delete(project.id()).await.unwrap();
    assert!(index.get(&artifact.id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_reconnect_sqlite_artifacts_still_available() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");

    let project = Project::new("Test Project".to_string());
    let artifact = make_artifact(ArtifactKind::LogFile, "recon");

    {
        let pool = connect_sqlite(&db_path).await.unwrap();
        let repo = SqliteProjectRepository::new(pool.clone());
        let index = SqliteArtifactIndex::new(pool.clone());

        repo.create(project.clone()).await.unwrap();
        index.add(project.id(), &artifact).await.unwrap();
        pool.close().await;
    }

    // 10. reconnect SQLite -> artifacts still available
    {
        let pool = connect_sqlite(&db_path).await.unwrap();
        let index = SqliteArtifactIndex::new(pool.clone());
        let fetched = index.get(&artifact.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, artifact.id);
    }
}

#[tokio::test]
async fn test_corrupted_location_kind_returns_error() {
    let (pool, index, _repo, project) = setup_db().await;
    let artifact = make_artifact(ArtifactKind::LogFile, "corr");

    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, created_at, updated_at, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(artifact.id.to_string())
    .bind(project.id().to_string())
    .bind("\"InvalidKind\"") 
    .bind("StorageKey") 
    .bind("test-corr")
    .bind(100)
    .bind("2024-01-01T00:00:00Z")
    .bind("2024-01-01T00:00:00Z")
    .bind("ready")
    .execute(&pool)
    .await
    .unwrap();

    // 11. corrupted location_kind/kind returns error, not fallback
    let result = index.get(&artifact.id).await;
    assert!(result.is_err(), "Expected error due to corrupted kind");
    match result.unwrap_err() {
        ports::error::PortError::InvalidStoredData { message, .. } => {
            assert!(message.contains("Invalid artifact kind"));
        }
        _ => panic!("Expected InvalidStoredData PortError"),
    }
}
