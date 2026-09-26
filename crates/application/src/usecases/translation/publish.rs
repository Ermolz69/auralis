use super::{
    InspectTranslationSourceRequest, InspectTranslationSourceUseCase, PublishTranslationRequest,
    linked_result_run,
};
use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use domain::media::ArtifactKind;
use ports::error::PortError;
use ports::storage::ArtifactStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{
    PublicationIntent, TranslationLinkStore, TranslationPublicationRecord,
    TranslationPublicationState,
};
use ports::translation_result::TranslationResultPort;
use std::sync::Arc;

#[derive(Clone)]
pub struct PublishTranslationResultUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    results: Arc<dyn TranslationResultPort>,
    artifacts: Arc<dyn ArtifactStore>,
}

impl PublishTranslationResultUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        results: Arc<dyn TranslationResultPort>,
        artifacts: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            inspector,
            links,
            results,
            artifacts,
        }
    }

    pub async fn execute(
        &self,
        request: PublishTranslationRequest,
    ) -> Result<TranslationPublicationRecord, ApplicationError> {
        let link = self
            .links
            .get(request.translation_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: "translation link".into(),
            })?;
        if link.project_id != request.project_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation belongs to another project".into(),
            });
        }
        let run_id = linked_result_run::resolve(self.links.as_ref(), &link).await?;
        if let Some(existing) = self
            .links
            .get_publication(request.translation_id, request.result_id)
            .await?
        {
            if existing.source_sha256 != link.source_sha256
                || (run_id != existing.run_id && link.selected_result_id != Some(request.result_id))
                || existing.state == TranslationPublicationState::Failed
            {
                return Err(ApplicationError::InvalidOperation {
                    message: "Stored publication is failed or no longer matches the project link"
                        .into(),
                });
            }
            return Ok(existing);
        }
        let (format, filename_hint) = match link.source_format.as_str() {
            "srt" => (TranslationSourceFormat::Srt, "translated.srt"),
            "vtt" => (TranslationSourceFormat::Vtt, "translated.vtt"),
            _ => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Translation format is unsupported".into(),
                });
            }
        };
        let (source_path, inspection) = self
            .inspector
            .inspect_with_path(InspectTranslationSourceRequest {
                project_id: request.project_id.clone(),
                source_artifact_id: link.source_artifact_id.clone(),
                format,
            })
            .await?;
        if inspection.source_sha256 != link.source_sha256 {
            return Err(ApplicationError::InvalidOperation {
                message: "Original source differs from translation link".into(),
            });
        }
        let result = self
            .results
            .verified_result(&source_path, &link, run_id, request.result_id)
            .await?;
        if result.result_id != request.result_id || result.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Verified result does not match the active run".into(),
            });
        }
        let temporary = tempfile::tempdir().map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        let output_path = temporary.path().join(filename_hint);
        tokio::fs::write(&output_path, &result.output_bytes)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        let staged = self
            .artifacts
            .import_external_file(
                &request.project_id,
                ArtifactKind::TranslatedTranscript,
                &output_path,
                Some(filename_hint),
            )
            .await?;
        let staging_key = staged.staging_key.clone();
        match self
            .artifacts
            .verify_storage_digest(&staging_key, staged.size_bytes, &result.output_sha256)
            .await
        {
            Ok(true) => {}
            Ok(false) => {
                return Err(self
                    .cleanup_staging(
                        &staging_key,
                        ApplicationError::InvalidOperation {
                            message: "Staged translated bytes differ from the verified result"
                                .into(),
                        },
                    )
                    .await);
            }
            Err(error) => {
                return Err(self.cleanup_staging(&staging_key, error.into()).await);
            }
        }
        let intent = PublicationIntent {
            translation_id: request.translation_id,
            result_id: request.result_id,
            result_revision: result.result_revision,
            run_id,
            artifact_id: staged.artifact.id.clone(),
            source_sha256: link.source_sha256,
            expected_link_revision: link.revision,
            needs_review: result.needs_review,
            created_at: domain::chrono::Utc::now(),
        };
        if let Err(error) = self.links.commit_staged_publication(intent, staged).await {
            return Err(self.cleanup_staging(&staging_key, error.into()).await);
        }
        self.links
            .get_publication(request.translation_id, request.result_id)
            .await?
            .ok_or_else(|| ApplicationError::Unexpected("Committed publication is missing".into()))
    }

    async fn cleanup_staging(&self, key: &str, primary: ApplicationError) -> ApplicationError {
        let mut cleanup = CleanupReport::new();
        if let Err(error) = self.artifacts.delete_storage_key(key).await {
            cleanup.add_failure(CleanupTarget::staging(key.to_owned()), error);
        }
        cleanup.into_error(primary)
    }
}
