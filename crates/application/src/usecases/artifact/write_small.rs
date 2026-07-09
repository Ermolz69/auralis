use domain::media::{Artifact, ArtifactKind};
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct WriteProjectArtifactRequest {
    pub project_id: ProjectId,
    pub kind: ArtifactKind,
    pub filename_hint: Option<String>,
    pub extension: String,
    pub data: Vec<u8>,
}

pub struct WriteProjectArtifactUseCase<P, S>
where
    P: ProjectRepository,
    S: ArtifactStore,
{
    project_repo: P,
    artifact_store: S,
}

impl<P, S> WriteProjectArtifactUseCase<P, S>
where
    P: ProjectRepository,
    S: ArtifactStore,
{
    pub fn new(project_repo: P, artifact_store: S) -> Self {
        Self {
            project_repo,
            artifact_store,
        }
    }

    pub async fn execute(
        &self,
        request: WriteProjectArtifactRequest,
    ) -> Result<(Project, Artifact), ApplicationError> {
        let safe_filename = request
            .filename_hint
            .unwrap_or_else(|| format!("artifact.{}", request.extension));

        let safe_filename = if safe_filename.ends_with(&format!(".{}", request.extension)) {
            safe_filename
        } else {
            format!("{}.{}", safe_filename, request.extension)
        };

        let artifact = self
            .artifact_store
            .write_small_artifact(
                &request.project_id,
                request.kind,
                &safe_filename,
                &request.data,
            )
            .await?;

        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        project.add_artifact(artifact.clone());
        self.project_repo.save(&project).await?;

        Ok((project, artifact))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use adapters_storage::local::LocalArtifactStore;
    use adapters_storage::sqlite::{SqliteProjectRepository, connect_sqlite};

    #[tokio::test]
    async fn test_write_small_artifact_mvp_flow() {
        let temp_dir = tempdir().unwrap();
        
        // Setup SQLite
        let db_path = temp_dir.path().join("test.sqlite");
        let pool = connect_sqlite(&db_path).await.unwrap();
        let repo = std::sync::Arc::new(SqliteProjectRepository::new(pool.clone()));
        
        // Setup Artifact Store
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        // Create Project
        let project = Project::new("Test MVP".to_string());
        repo.create(project.clone()).await.unwrap();

        // Run Use Case
        let use_case = WriteProjectArtifactUseCase::new(repo.clone(), store);
        
        let req = WriteProjectArtifactRequest {
            project_id: project.id().clone(),
            kind: ArtifactKind::LogFile,
            filename_hint: Some("my_log".to_string()),
            extension: "txt".to_string(),
            data: b"hello mvp".to_vec(),
        };

        let (saved_project, artifact) = use_case.execute(req).await.unwrap();

        // 1. write_small_artifact returns StorageKey, not LocalPath
        match &artifact.location {
            domain::media::ArtifactLocation::StorageKey(key) => {
                assert!(key.contains("log-file"));
                assert!(key.ends_with(".txt"));
            }
            _ => panic!("Expected StorageKey"),
        }

        // 3. register/write artifact adds artifact into project.artifacts
        assert_eq!(saved_project.artifacts().len(), 1);

        // 4. SqliteProjectRepository saves artifacts_json
        let reloaded = repo.get(project.id()).await.unwrap().unwrap();
        assert_eq!(reloaded.artifacts().len(), 1);

        // 5. after reconnect project still has artifacts
        drop(repo);
        pool.close().await;
        
        let pool2 = connect_sqlite(&db_path).await.unwrap();
        let repo2 = SqliteProjectRepository::new(pool2.clone());
        let reloaded2 = repo2.get(project.id()).await.unwrap().unwrap();
        assert_eq!(reloaded2.artifacts().len(), 1);

        // 2. resolve_artifact(StorageKey) returns path under base_dir
        let store2 = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let resolved_path = store2.resolve_artifact(&artifact).await.unwrap();
        assert!(resolved_path.starts_with(temp_dir.path()));
        assert!(resolved_path.exists());
        
        let content = tokio::fs::read(&resolved_path).await.unwrap();
        assert_eq!(content, b"hello mvp");

        // 6. delete project removes project row, but пока не удаляет files
        repo2.delete(project.id()).await.unwrap();
        let reloaded_deleted = repo2.get(project.id()).await.unwrap();
        assert!(reloaded_deleted.is_none());
        
        // File should still exist
        assert!(resolved_path.exists());
    }
}
