#![allow(clippy::unwrap_used, clippy::expect_used)]
use sqlx::SqlitePool;
use tempfile::tempdir;

use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation};
use domain::project::Project;

use super::repository::SqliteArtifactIndex;
use crate::sqlite::{SqliteProjectRepository, connect_sqlite};
use ports::repository::ProjectRepository;

pub async fn setup_db() -> (
    SqlitePool,
    SqliteArtifactIndex,
    SqliteProjectRepository,
    Project,
) {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.sqlite");
    let pool = connect_sqlite(&db_path).await.unwrap();
    let artifact_index = SqliteArtifactIndex::new(pool.clone());
    let repo = SqliteProjectRepository::new(pool.clone());

    let project = Project::new("Test Project".to_string());
    repo.create(project.clone()).await.unwrap();

    (pool, artifact_index, repo, project)
}

pub fn make_artifact(kind: ArtifactKind, seed: &str) -> Artifact {
    Artifact {
        id: ArtifactId::new(),
        kind,
        location: ArtifactLocation::StorageKey(format!("test-key-{}.txt", seed)),
        size_bytes: Some(100),
        state: domain::media::ArtifactState::Ready,
        created_at: domain::chrono::Utc::now(),
        updated_at: domain::chrono::Utc::now(),
        ready_at: Some(domain::chrono::Utc::now()),
    }
}
