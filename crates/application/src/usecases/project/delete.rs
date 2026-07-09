use std::sync::Arc;

use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;

pub struct DeleteProjectRequest {
    pub project_id: ProjectId,
}

pub struct DeleteProjectUseCase {
    project_repo: Arc<dyn ProjectRepository>,
    artifact_index: Arc<dyn ArtifactIndex>,
    artifact_store: Arc<dyn ArtifactStore>,
}

impl DeleteProjectUseCase {
    pub fn new(
        project_repo: Arc<dyn ProjectRepository>,
        artifact_index: Arc<dyn ArtifactIndex>,
        artifact_store: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            project_repo,
            artifact_index,
            artifact_store,
        }
    }

    pub async fn execute(&self, request: DeleteProjectRequest) -> Result<(), ApplicationError> {
        let project_id = &request.project_id;

        // 1. List all artifacts for the project
        let artifacts = self.artifact_index.list_by_project(project_id).await?;

        // 2. Delete physical files
        for artifact in artifacts {
            if let Err(e) = self.artifact_store.delete_artifact(&artifact).await {
                // We should probably log this but continue deleting other artifacts
                eprintln!(
                    "WARNING: Failed to delete artifact {} for project {}: {}",
                    artifact.id, project_id, e
                );
            }
        }

        // Delete the project directory itself
        if let Err(e) = self.artifact_store.delete_project_dir(project_id).await {
            eprintln!(
                "WARNING: Failed to delete project directory for project {}: {}",
                project_id, e
            );
        }

        // 3. Delete the project via ProjectRepository
        // 4. SQLite CASCADE will delete artifact rows in the index automatically
        self.project_repo.delete(project_id).await?;

        Ok(())
    }
}
