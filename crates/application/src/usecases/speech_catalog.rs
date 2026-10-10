use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::speech_catalog::{SpeechCatalogPort, SpeechCatalogSnapshot};
use std::sync::Arc;

use crate::error::ApplicationError;

pub struct SpeechCatalogUseCase {
    projects: Arc<dyn ProjectRepository>,
    catalog: Option<Arc<dyn SpeechCatalogPort>>,
}

impl SpeechCatalogUseCase {
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        catalog: Option<Arc<dyn SpeechCatalogPort>>,
    ) -> Self {
        Self { projects, catalog }
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
                message: "TTS component is not installed".into(),
            })?;
        Ok(catalog.snapshot(project).await?)
    }
}
