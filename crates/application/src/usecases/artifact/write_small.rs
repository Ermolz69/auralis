use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct WriteProjectArtifactRequest {
    pub project_id: ProjectId,
    pub kind: ArtifactKind,
    pub filename_hint: Option<String>,
    pub extension: String,
    pub data: Vec<u8>,
}

pub struct WriteProjectArtifactUseCase<I, S>
where
    I: ArtifactIndex,
    S: ArtifactStore,
{
    artifact_index: I,
    artifact_store: S,
}

impl<I, S> WriteProjectArtifactUseCase<I, S>
where
    I: ArtifactIndex,
    S: ArtifactStore,
{
    pub fn new(artifact_index: I, artifact_store: S) -> Self {
        Self {
            artifact_index,
            artifact_store,
        }
    }

    pub async fn execute(
        &self,
        request: WriteProjectArtifactRequest,
    ) -> Result<Artifact, ApplicationError> {
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

        self.artifact_index
            .add(&request.project_id, &artifact)
            .await?;

        Ok(artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::local::LocalArtifactStore;
    use adapters_storage::sqlite::{SqliteArtifactIndex, SqliteProjectRepository, connect_sqlite};
    use domain::project::Project;
    use ports::repository::ProjectRepository;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_write_small_artifact_mvp_flow() {
        let temp_dir = tempdir().unwrap();

        // Setup SQLite
        let db_path = temp_dir.path().join("test.sqlite");
        let pool = connect_sqlite(&db_path).await.unwrap();
        let artifact_index = SqliteArtifactIndex::new(pool.clone());
        let repo = SqliteProjectRepository::new(pool.clone());

        // Setup Artifact Store
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        // Create Project (needed for foreign key constraint)
        let project = Project::new("Test MVP".to_string());
        repo.create(project.clone()).await.unwrap();

        // Run Use Case
        let use_case = WriteProjectArtifactUseCase::new(artifact_index, store);

        let req = WriteProjectArtifactRequest {
            project_id: project.id().clone(),
            kind: ArtifactKind::LogFile,
            filename_hint: Some("my_log".to_string()),
            extension: "txt".to_string(),
            data: b"hello mvp".to_vec(),
        };

        let artifact = use_case.execute(req).await.unwrap();

        // 1. write_small_artifact returns StorageKey, not LocalPath
        match &artifact.location {
            domain::media::ArtifactLocation::StorageKey(key) => {
                assert!(key.contains("log-file"));
                assert!(key.ends_with(".txt"));
            }
            _ => panic!("Expected StorageKey"),
        }

        // 3. register/write artifact adds artifact into index
        let pool2 = connect_sqlite(&db_path).await.unwrap();
        let index2 = SqliteArtifactIndex::new(pool2.clone());
        let artifacts = index2.list_by_project(project.id()).await.unwrap();
        assert_eq!(artifacts.len(), 1);

        // 2. resolve_artifact(StorageKey) returns path under base_dir
        let store2 = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let resolved_path = store2.resolve_artifact(&artifact).await.unwrap();
        assert!(resolved_path.starts_with(temp_dir.path()));
        assert!(resolved_path.exists());

        let content = tokio::fs::read(&resolved_path).await.unwrap();
        assert_eq!(content, b"hello mvp");
    }
}
