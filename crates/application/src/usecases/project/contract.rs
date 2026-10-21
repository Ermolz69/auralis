use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::{
    error::PortError,
    puza_contract::{PuzaContractPort, PuzaContractSnapshot, PuzaVoiceBinding},
    repository::ProjectRepository,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct ProjectContractUseCase {
    pub projects: Arc<dyn ProjectRepository>,
    pub runtime: Option<Arc<dyn PuzaContractPort>>,
    pub unavailable_reason: String,
}

impl ProjectContractUseCase {
    pub async fn ensure(
        &self,
        project: &ProjectId,
    ) -> Result<PuzaContractSnapshot, ApplicationError> {
        self.projects
            .get(project)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project.clone()))?;
        let runtime = self.runtime.as_ref().ok_or_else(|| self.unavailable())?;
        if let Some(snapshot) = self.projects.contract_project(project).await? {
            let bindings: Vec<PuzaVoiceBinding> = serde_json::from_str(&snapshot.bindings_json)
                .map_err(|e| PortError::InvalidSource {
                    message: e.to_string(),
                })?;
            let checked = runtime
                .prepare_project(project, snapshot.source_text.clone(), bindings)
                .await?;
            if checked != snapshot {
                return Err(PortError::Conflict {
                    resource: "CONTRACT project".into(),
                    message: "The saved CONTRACT configuration changed".into(),
                }
                .into());
            }
            return Ok(snapshot);
        }
        let snapshot = runtime.prepare_project(project, None, Vec::new()).await?;
        self.projects
            .save_contract_project(project, None, &snapshot)
            .await?;
        Ok(snapshot)
    }

    pub async fn configure(
        &self,
        project: &ProjectId,
        text: String,
        bindings: Vec<PuzaVoiceBinding>,
    ) -> Result<PuzaContractSnapshot, ApplicationError> {
        let previous = self.ensure(project).await?;
        let snapshot = self
            .runtime
            .as_ref()
            .ok_or_else(|| self.unavailable())?
            .prepare_project(project, Some(text), bindings)
            .await?;
        self.projects
            .save_contract_project(project, Some(&previous.sha256), &snapshot)
            .await?;
        Ok(snapshot)
    }

    pub async fn configure_voice(
        &self,
        project: &ProjectId,
        binding: PuzaVoiceBinding,
    ) -> Result<PuzaVoiceBinding, ApplicationError> {
        let previous = self.ensure(project).await?;
        let mut bindings: Vec<PuzaVoiceBinding> = serde_json::from_str(&previous.bindings_json)
            .map_err(|error| PortError::InvalidSource {
                message: error.to_string(),
            })?;
        if let Some(existing) = bindings
            .iter_mut()
            .find(|value| value.role_id == binding.role_id)
        {
            *existing = binding.clone();
        } else {
            bindings.push(binding.clone());
        }
        let snapshot = self
            .runtime
            .as_ref()
            .ok_or_else(|| self.unavailable())?
            .prepare_project(project, previous.source_text.clone(), bindings)
            .await?;
        self.projects
            .save_contract_project(project, Some(&previous.sha256), &snapshot)
            .await?;
        Ok(binding)
    }
    pub async fn default_voice(
        &self,
        project: &ProjectId,
    ) -> Result<Option<PuzaVoiceBinding>, ApplicationError> {
        let snapshot = self.ensure(project).await?;
        let bindings: Vec<PuzaVoiceBinding> = serde_json::from_str(&snapshot.bindings_json)
            .map_err(|e| PortError::InvalidSource {
                message: e.to_string(),
            })?;
        Ok(bindings
            .into_iter()
            .find(|binding| binding.role_id == "narrator"))
    }
    fn unavailable(&self) -> PortError {
        PortError::Unsupported {
            message: self.unavailable_reason.clone(),
        }
    }
}
