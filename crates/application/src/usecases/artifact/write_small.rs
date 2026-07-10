use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::ArtifactStore;
use ports::transaction::{CommitStagedArtifactWrite, StorageUnitOfWork};

use crate::error::ApplicationError;
use tempfile::NamedTempFile;
use std::io::Write;

pub struct WriteProjectArtifactRequest {
    pub project_id: ProjectId,
    pub kind: ArtifactKind,
    pub filename_hint: Option<String>,
    pub extension: String,
    pub data: Vec<u8>,
}

pub struct WriteProjectArtifactUseCase<S, U>
where
    S: ArtifactStore + Clone + 'static,
    U: StorageUnitOfWork + Clone + 'static,
{
    artifact_store: S,
    storage_uow: U,
}

impl<S, U> WriteProjectArtifactUseCase<S, U>
where
    S: ArtifactStore + Clone + 'static,
    U: StorageUnitOfWork + Clone + 'static,
{
    pub fn new(artifact_store: S, storage_uow: U) -> Self {
        Self {
            artifact_store,
            storage_uow,
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

        // 1. Write data to a temporary file
        let mut temp_file = NamedTempFile::new().map_err(|e| PortError::Unexpected {
            message: format!("Failed to create temp file: {}", e),
        })?;
        temp_file.write_all(&request.data).map_err(|e| PortError::Unexpected {
            message: format!("Failed to write to temp file: {}", e),
        })?;
        let temp_path = temp_file.into_temp_path();

        // 2. Stage the file
        let staged = self
            .artifact_store
            .stage_owned_temp_file(
                &request.project_id,
                request.kind,
                &temp_path,
                Some(&safe_filename),
            )
            .await?;

        // 3. Commit the transaction to persist artifact & enqueue finalize outbox message
        let commit_res = self.storage_uow
            .commit_staged_artifact_write(CommitStagedArtifactWrite {
                project_id: request.project_id.clone(),
                artifact: staged.artifact.clone(),
                staging_key: staged.staging_key.clone(),
                final_key: staged.final_key.clone(),
                temp_path_to_delete: None,
            })
            .await;

        if let Err(e) = commit_res {
            let _ = self.artifact_store.delete_storage_key(&staged.staging_key).await;
            return Err(ApplicationError::Port(e));
        }

        Ok(staged.artifact)
    }
}