use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::speech_catalog::{SpeechCatalogPort, SpeechCatalogSnapshot};
use std::sync::Arc;

use crate::error::ApplicationError;

pub struct SpeechCatalogUseCase {
    projects: Arc<dyn ProjectRepository>,
    catalog: Option<Arc<dyn SpeechCatalogPort>>,
    unavailable_reason: String,
}

impl SpeechCatalogUseCase {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        catalog: Option<Arc<dyn SpeechCatalogPort>>,
    ) -> Self {
        Self {
            projects,
            catalog,
            unavailable_reason: "TTS component is not installed".into(),
        }
    }

    pub fn with_unavailable_reason(mut self, reason: String) -> Self {
        self.unavailable_reason = reason;
        self
    }

    pub async fn execute(
        &self,
        project: &ProjectId,
    ) -> Result<SpeechCatalogSnapshot, ApplicationError> {
        self.projects
            .get(project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        let catalog = self
            .catalog
            .as_ref()
            .ok_or_else(|| PortError::Unsupported {
                message: self.unavailable_reason.clone(),
            })?;
        Ok(catalog.snapshot(project).await?)
    }
}
