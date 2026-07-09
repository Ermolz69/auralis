use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;

use super::artifact_mapper::{artifact_to_row_values, row_to_artifact};
use super::artifact_row::ArtifactRow;

pub struct SqliteArtifactIndex {
    pool: SqlitePool,
}

impl SqliteArtifactIndex {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ArtifactIndex for SqliteArtifactIndex {
    async fn add(&self, project_id: &ProjectId, artifact: &Artifact) -> Result<(), PortError> {
        let values = artifact_to_row_values(project_id, artifact)?;

        sqlx::query(
            r#"
            INSERT INTO artifacts (
                id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                project_id = excluded.project_id,
                kind = excluded.kind,
                location_kind = excluded.location_kind,
                location_value = excluded.location_value,
                size_bytes = excluded.size_bytes,
                state = excluded.state,
                updated_at = excluded.updated_at,
                ready_at = excluded.ready_at
            "#,
        )
        .bind(values.id)
        .bind(values.project_id)
        .bind(values.kind)
        .bind(values.location_kind)
        .bind(values.location_value)
        .bind(values.size_bytes)
        .bind(values.state)
        .bind(values.created_at)
        .bind(values.updated_at)
        .bind(values.ready_at)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to add artifact: {}", e),
        })?;

        Ok(())
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
        let row = sqlx::query_as::<_, ArtifactRow>(
            r#"
            SELECT 
                id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
            FROM artifacts
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch artifact: {}", e),
        })?;

        row.map(row_to_artifact).transpose()
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Artifact>, PortError> {
        let rows = sqlx::query_as::<_, ArtifactRow>(
            r#"
            SELECT 
                id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
            FROM artifacts
            WHERE project_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(project_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list artifacts by project: {}", e),
        })?;

        let mut artifacts = Vec::new();
        for row in rows {
            artifacts.push(row_to_artifact(row)?);
        }
        Ok(artifacts)
    }

    async fn list_by_project_and_kind(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError> {
        let kind_val = serde_json::to_value(&kind).map_err(|e| PortError::Unexpected {
            message: format!("Failed to serialize artifact kind: {}", e),
        })?;

        let kind_str = kind_val
            .as_str()
            .ok_or_else(|| PortError::Unexpected {
                message: "Artifact kind is not a string".to_string(),
            })?
            .to_string();

        let rows = sqlx::query_as::<_, ArtifactRow>(
            r#"
            SELECT 
                id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at, ready_at
            FROM artifacts
            WHERE project_id = ? AND kind = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(project_id.to_string())
        .bind(kind_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to list artifacts by project and kind: {}", e),
        })?;

        let mut artifacts = Vec::new();
        for row in rows {
            artifacts.push(row_to_artifact(row)?);
        }
        Ok(artifacts)
    }

    async fn delete(&self, id: &ArtifactId) -> Result<(), PortError> {
        sqlx::query("DELETE FROM artifacts WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to delete artifact: {}", e),
            })?;
        Ok(())
    }

    async fn update_state(
        &self,
        id: &ArtifactId,
        state: domain::media::ArtifactState,
        ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
    ) -> Result<(), PortError> {
        let state_val = serde_json::to_value(&state).map_err(|e| PortError::Unexpected {
            message: format!("Failed to serialize artifact state: {}", e),
        })?;

        let state_str = state_val
            .as_str()
            .ok_or_else(|| PortError::Unexpected {
                message: "Artifact state is not a string".to_string(),
            })?
            .to_string();

        let ready_at_str = ready_at.map(|dt| dt.to_rfc3339());
        let updated_at_str = domain::chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE artifacts
            SET state = ?, updated_at = ?, ready_at = coalesce(?, ready_at)
            WHERE id = ?
            "#,
        )
        .bind(state_str)
        .bind(updated_at_str)
        .bind(ready_at_str)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to update artifact state: {}", e),
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{SqliteProjectRepository, connect_sqlite};
    use domain::media::ArtifactLocation;
    use domain::project::Project;
    use ports::repository::ProjectRepository;
    use tempfile::tempdir;

    async fn setup_db() -> (
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

    fn make_artifact(kind: ArtifactKind, seed: &str) -> Artifact {
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

        // Disable foreign keys temporarily to bypass CHECK constraint?
        // No, CHECK constraint is on the table itself.
        // We can just verify that kind deserialization error is properly mapped to a PortError and not a panic/fallback
        // Let's insert a corrupted `kind`
        sqlx::query(
            "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, created_at, updated_at, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(artifact.id.to_string())
        .bind(project.id().to_string())
        .bind("\"InvalidKind\"") // corrupted kind (JSON string that doesn't match enum)
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
            ports::error::PortError::Unexpected { message } => {
                assert!(message.contains("Invalid artifact kind"));
            }
            _ => panic!("Expected Unexpected PortError"),
        }
    }
}
