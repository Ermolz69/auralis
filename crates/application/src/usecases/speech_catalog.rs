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
    contract: Option<Arc<super::project::contract::ProjectContractUseCase>>,
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
            contract: None,
        }
    }

    pub fn with_unavailable_reason(mut self, reason: String) -> Self {
        self.unavailable_reason = reason;
        self
    }

    pub fn with_contract(
        mut self,
        contract: Arc<super::project::contract::ProjectContractUseCase>,
    ) -> Self {
        self.contract = Some(contract);
        self
    }

    pub async fn execute(
        &self,
        project: &ProjectId,
    ) -> Result<SpeechCatalogSnapshot, ApplicationError> {
        if let Some(contract) = &self.contract {
            contract.ensure(project).await?;
        }
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
