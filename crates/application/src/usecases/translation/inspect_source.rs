use super::InspectTranslationSourceRequest;
use crate::error::ApplicationError;
use domain::media::{ArtifactKind, ArtifactLocation, ArtifactState};
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::translation_inspection::{TranslationInspection, TranslationInspectorPort};
use std::path::PathBuf;
use std::sync::Arc;

pub struct InspectTranslationSourceUseCase {
    projects: Arc<dyn ProjectRepository>,
    artifacts: Arc<dyn ArtifactIndex>,
    artifact_store: Arc<dyn ArtifactStore>,
    inspector: Arc<dyn TranslationInspectorPort>,
}

impl InspectTranslationSourceUseCase {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        artifacts: Arc<dyn ArtifactIndex>,
        artifact_store: Arc<dyn ArtifactStore>,
        inspector: Arc<dyn TranslationInspectorPort>,
    ) -> Self {
        Self {
            projects,
            artifacts,
            artifact_store,
            inspector,
        }
    }

    pub async fn execute(
        &self,
        request: InspectTranslationSourceRequest,
    ) -> Result<TranslationInspection, ApplicationError> {
        self.inspect_with_path(request)
            .await
            .map(|(_, report)| report)
    }

    pub(crate) async fn inspect_with_path(
        &self,
        request: InspectTranslationSourceRequest,
    ) -> Result<(PathBuf, TranslationInspection), ApplicationError> {
        self.projects
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;
        let original = self
            .artifacts
            .list_by_project_and_kind(&request.project_id, ArtifactKind::OriginalSubtitle)
            .await?
            .into_iter()
            .find(|artifact| artifact.id == request.source_artifact_id)
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Original subtitle artifact does not belong to this project".into(),
            })?;
        if original.state != ArtifactState::Ready
            || !matches!(&original.location, ArtifactLocation::StorageKey(_))
        {
            return Err(ApplicationError::InvalidOperation {
                message: "Original subtitle artifact is not a ready managed file".into(),
            });
        }
        let source_path = self.artifact_store.resolve_artifact(&original).await?;
        let inspection = self.inspector.inspect(&source_path, request.format).await?;
        if original
            .size_bytes
            .is_some_and(|expected| expected != inspection.source_size)
        {
            return Err(ApplicationError::InvalidOperation {
                message: "Original subtitle artifact size differs from the indexed value".into(),
            });
        }
        Ok((source_path, inspection))
    }
}
