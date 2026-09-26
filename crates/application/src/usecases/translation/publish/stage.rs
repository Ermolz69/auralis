use super::use_case::PublishTranslationResultUseCase;
use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use crate::usecases::translation::PublishTranslationRequest;
use domain::media::ArtifactKind;
use ports::error::PortError;
use ports::translation_links::{PublicationIntent, TranslationLink, TranslationPublicationRecord};
use ports::translation_result::VerifiedTranslationResult;

impl PublishTranslationResultUseCase {
    pub(super) async fn stage(
        &self,
        request: PublishTranslationRequest,
        link: TranslationLink,
        result: VerifiedTranslationResult,
        expected_link_revision: u64,
    ) -> Result<TranslationPublicationRecord, ApplicationError> {
        let run_id = result.run_id;
        let filename_hint = if link.source_format == "vtt" {
            "translated.vtt"
        } else {
            "translated.srt"
        };
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
            expected_link_revision,
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
