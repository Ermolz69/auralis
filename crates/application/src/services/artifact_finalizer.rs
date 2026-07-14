use domain::media::{ArtifactId, ArtifactState};
use ports::artifact_index::ArtifactIndex;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

#[derive(Clone)]
pub struct ArtifactFinalizer<I, S> {
    artifact_index: I,
    artifact_store: S,
}

impl<I, S> ArtifactFinalizer<I, S>
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

    pub async fn finalize(
        &self,
        artifact_id: &ArtifactId,
        staging_key: &str,
        final_key: &str,
    ) -> Result<bool, ApplicationError> {
        let artifact = self.artifact_index.get(artifact_id).await?;
        let artifact = match artifact {
            Some(a) => a,
            None => {
                // Project or artifact was deleted. Do not finalize, just cleanup staging.
                let _ = self.artifact_store.delete_storage_key(staging_key).await;
                return Ok(false);
            }
        };

        if artifact.state == ArtifactState::Ready {
            // Already finalized
            return Ok(true);
        }

        // 1. Move file
        self.artifact_store
            .finalize_staged_artifact(staging_key, final_key)
            .await?;

        // 2. Update artifact state in index
        self.artifact_index
            .update_state(
                artifact_id,
                ArtifactState::Ready,
                Some(domain::chrono::Utc::now()),
            )
            .await?;

        Ok(true)
    }
}
