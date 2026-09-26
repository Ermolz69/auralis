use super::{
    InspectTranslationSourceRequest, InspectTranslationSourceUseCase,
    SelectTranslationResultRequest,
};
use crate::error::ApplicationError;
use domain::media::{ArtifactKind, ArtifactLocation, ArtifactState};
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;
use ports::storage::ArtifactStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{
    SelectTranslationHistoryResult, TranslationLink, TranslationLinkStore,
    TranslationPublicationState,
};
use ports::translation_result::TranslationResultPort;
use std::sync::Arc;

pub struct SelectTranslationResultUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    results: Arc<dyn TranslationResultPort>,
    artifacts: Arc<dyn ArtifactIndex>,
    files: Arc<dyn ArtifactStore>,
}

impl SelectTranslationResultUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        results: Arc<dyn TranslationResultPort>,
        artifacts: Arc<dyn ArtifactIndex>,
        files: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            inspector,
            links,
            results,
            artifacts,
            files,
        }
    }

    pub async fn execute(
        &self,
        request: SelectTranslationResultRequest,
    ) -> Result<TranslationLink, ApplicationError> {
        let link = self
            .links
            .get(request.translation_id)
            .await?
            .filter(|link| link.project_id == request.project_id)
            .ok_or_else(|| PortError::NotFound {
                resource: "project translation".into(),
            })?;
        let publication = self
            .links
            .get_publication(request.translation_id, request.result_id)
            .await?
            .filter(|record| {
                record.state == TranslationPublicationState::Ready
                    && record.source_sha256 == link.source_sha256
            })
            .ok_or_else(|| PortError::Conflict {
                resource: "translation history".into(),
                message: "Result is not a ready publication for this source".into(),
            })?;
        let intent = self
            .links
            .run_intent(publication.run_id)
            .await?
            .filter(|intent| intent.translation_id == request.translation_id)
            .ok_or_else(|| PortError::NotFound {
                resource: "historical frozen run".into(),
            })?;
        let format = match link.source_format.as_str() {
            "srt" => TranslationSourceFormat::Srt,
            "vtt" => TranslationSourceFormat::Vtt,
            _ => {
                return Err(PortError::InvalidSource {
                    message: "Unsupported historical source format".into(),
                }
                .into());
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
        if inspection.source_sha256 != link.source_sha256
            || inspection.parser_version != intent.parser_version
        {
            return Err(PortError::Conflict {
                resource: "translation source".into(),
                message: "Original source or parser differs from the historical run".into(),
            }
            .into());
        }
        let result = self
            .results
            .verified_result(&source_path, &link, publication.run_id, request.result_id)
            .await?;
        if result.result_id != request.result_id
            || result.run_id != publication.run_id
            || result.result_revision != publication.result_revision
            || result.needs_review != publication.needs_review
        {
            return Err(PortError::Conflict {
                resource: "translation history".into(),
                message: "Verified result differs from publication metadata".into(),
            }
            .into());
        }
        let artifact = self
            .artifacts
            .list_by_project_and_kind(&request.project_id, ArtifactKind::TranslatedTranscript)
            .await?
            .into_iter()
            .find(|artifact| {
                artifact.id == publication.artifact_id && artifact.state == ArtifactState::Ready
            })
            .ok_or_else(|| PortError::NotFound {
                resource: "ready historical artifact".into(),
            })?;
        let ArtifactLocation::StorageKey(key) = artifact.location else {
            return Err(PortError::InvalidSource {
                message: "Historical output must be a managed file".into(),
            }
            .into());
        };
        let size =
            u64::try_from(result.output_bytes.len()).map_err(|_| PortError::InvalidSource {
                message: "Historical output is too large".into(),
            })?;
        if artifact.size_bytes != Some(size)
            || !self
                .files
                .verify_storage_digest(&key, size, &result.output_sha256)
                .await?
        {
            return Err(PortError::Conflict {
                resource: "translation output".into(),
                message: "Historical output bytes differ from the verified result".into(),
            }
            .into());
        }
        self.links
            .select_historical_result(SelectTranslationHistoryResult {
                project_id: request.project_id,
                translation_id: request.translation_id,
                result_id: request.result_id,
                expected_link_revision: request.expected_link_revision,
                updated_at: domain::chrono::Utc::now(),
            })
            .await
            .map_err(Into::into)
    }
}
