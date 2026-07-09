use domain::media::Artifact;
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct RegisterArtifactUseCase<P, S>
where
    P: ProjectRepository,
    S: ArtifactStore,
{
    project_repo: P,
    artifact_store: S,
}

impl<P, S> RegisterArtifactUseCase<P, S>
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
        project_id: ProjectId,
        artifact: Artifact,
    ) -> Result<Project, ApplicationError> {
        self.artifact_store
            .register_artifact(&project_id, &artifact)
            .await?;

        let mut project = self
            .project_repo
            .get(&project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project_id.clone()))?;

        project.add_artifact(artifact);
        self.project_repo.save(&project).await?;

        Ok(project)
    }
}
