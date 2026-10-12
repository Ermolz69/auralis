use crate::error::ApplicationError;
use domain::media::{ArtifactId, ArtifactLocation, ArtifactState};
use ports::artifact_index::ArtifactIndex;
use ports::storage::ArtifactStore;
use std::path::PathBuf;

pub struct ResolveArtifactPathRequest {
    pub artifact_id: ArtifactId,
}

pub struct ResolveArtifactPathResponse {
    pub absolute_path: PathBuf,
    pub managed_ready: bool,
}

pub struct ResolveArtifactPathUseCase<I: ArtifactIndex, S: ArtifactStore> {
    index: I,
    store: S,
}

impl<I: ArtifactIndex, S: ArtifactStore> ResolveArtifactPathUseCase<I, S> {
    pub fn new(index: I, store: S) -> Self {
        Self { index, store }
    }

    pub async fn execute(
        &self,
        req: ResolveArtifactPathRequest,
    ) -> Result<ResolveArtifactPathResponse, ApplicationError> {
        let artifact = self.index.get(&req.artifact_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Artifact not found".into(),
            }
        })?;

        let absolute_path = self.store.resolve_artifact(&artifact).await?;

        let managed_ready = artifact.state == ArtifactState::Ready
            && matches!(artifact.location, ArtifactLocation::StorageKey(_));
        Ok(ResolveArtifactPathResponse {
            absolute_path,
            managed_ready,
        })
    }
}
