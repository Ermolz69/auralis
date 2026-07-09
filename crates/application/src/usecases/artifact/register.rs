use domain::media::Artifact;
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct RegisterArtifactUseCase<I, S>
where
    I: ArtifactIndex,
    S: ArtifactStore,
{
    artifact_index: I,
    artifact_store: S,
}

impl<I, S> RegisterArtifactUseCase<I, S>
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
        project_id: ProjectId,
        artifact: Artifact,
    ) -> Result<(), ApplicationError> {
        self.artifact_store
            .register_artifact(&project_id, &artifact)
            .await?;

        self.artifact_index.add(&project_id, &artifact).await?;

        Ok(())
    }
}
