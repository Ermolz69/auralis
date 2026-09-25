use super::ImportTranslationSourceRequest;
use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use domain::media::{Artifact, ArtifactKind};
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::transaction::{CommitStagedArtifactWrite, StorageUnitOfWork};
use ports::translation_inspection::{TranslationInspectorPort, TranslationSourceFormat};
use std::sync::Arc;

pub struct ImportTranslationSourceUseCase {
    projects: Arc<dyn ProjectRepository>,
    artifacts: Arc<dyn ArtifactStore>,
    storage: Arc<dyn StorageUnitOfWork>,
    inspector: Arc<dyn TranslationInspectorPort>,
}

impl ImportTranslationSourceUseCase {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        artifacts: Arc<dyn ArtifactStore>,
        storage: Arc<dyn StorageUnitOfWork>,
        inspector: Arc<dyn TranslationInspectorPort>,
    ) -> Self {
        Self {
            projects,
            artifacts,
            storage,
            inspector,
        }
    }

    pub async fn execute(
        &self,
        request: ImportTranslationSourceRequest,
    ) -> Result<Artifact, ApplicationError> {
        let extension = request
            .path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(
            (request.format, extension.as_deref()),
            (TranslationSourceFormat::Srt, Some("srt"))
                | (TranslationSourceFormat::Vtt, Some("vtt"))
        ) {
            return Err(ApplicationError::InvalidOperation {
                message: "Subtitle filename extension differs from requested format".into(),
            });
        }
        self.projects
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;
        let inspection = self
            .inspector
            .inspect(&request.path, request.format)
            .await?;
        let filename = request.path.file_name().and_then(|value| value.to_str());
        let staged = self
            .artifacts
            .import_external_file(
                &request.project_id,
                ArtifactKind::OriginalSubtitle,
                &request.path,
                filename,
            )
            .await?;
        let staging_key = staged.staging_key.clone();
        let commit = async {
            if !self
                .artifacts
                .verify_staging_digest(
                    &staging_key,
                    inspection.source_size,
                    &inspection.source_sha256,
                )
                .await?
            {
                return Err(ApplicationError::InvalidOperation {
                    message: "Staged subtitle differs from inspected source".into(),
                });
            }
            self.storage
                .commit_staged_artifact_write(CommitStagedArtifactWrite {
                    project_id: request.project_id,
                    artifact: staged.artifact.clone(),
                    staging_key: staging_key.clone(),
                    final_key: staged.final_key,
                    temp_workspace_key: None,
                })
                .await?;
            Ok(staged.artifact)
        }
        .await;
        match commit {
            Ok(artifact) => Ok(artifact),
            Err(error) => {
                let mut cleanup = CleanupReport::new();
                if let Err(cleanup_error) = self.artifacts.delete_storage_key(&staging_key).await {
                    cleanup.add_failure(CleanupTarget::staging(staging_key), cleanup_error);
                }
                Err(cleanup.into_error(error))
            }
        }
    }
}
