use domain::project::Project;
use ports::repository::ProjectRepository;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct CreateProjectRequest {
    pub title: String,
}

#[derive(Debug)]
pub struct CreateProjectResponse {
    pub project: Project,
}

pub struct CreateProjectUseCase<R: ProjectRepository> {
    project_repo: R,
    contract: Option<std::sync::Arc<dyn ports::puza_contract::PuzaContractPort>>,
    required_contract: Option<String>,
    media: Option<std::sync::Arc<dyn ports::media_project::MediaProjectStore>>,
}

impl<R: ProjectRepository> CreateProjectUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self {
            project_repo,
            contract: None,
            required_contract: None,
            media: None,
        }
    }

    pub fn with_media(
        mut self,
        media: std::sync::Arc<dyn ports::media_project::MediaProjectStore>,
    ) -> Self {
        self.media = Some(media);
        self
    }
    pub fn with_contract(
        mut self,
        contract: Option<std::sync::Arc<dyn ports::puza_contract::PuzaContractPort>>,
        reason: String,
    ) -> Self {
        self.contract = contract;
        self.required_contract = Some(reason);
        self
    }

    pub async fn execute(
        &self,
        request: CreateProjectRequest,
    ) -> Result<CreateProjectResponse, ApplicationError> {
        let project = Project::new(request.title)?;
        let created_project = if let Some(reason) = &self.required_contract {
            let contract =
                self.contract
                    .as_ref()
                    .ok_or_else(|| ports::error::PortError::Unsupported {
                        message: reason.clone(),
                    })?;
            let snapshot = contract
                .prepare_project(project.id(), None, Vec::new())
                .await?;
            self.project_repo
                .create_with_contract(project, snapshot)
                .await?
        } else {
            self.project_repo.create(project).await?
        };

        if let Some(media) = &self.media
            && let Err(error) = media.ensure(&created_project.id().to_string()).await
        {
            self.project_repo.delete(created_project.id()).await?;
            return Err(error.into());
        }
        Ok(CreateProjectResponse {
            project: created_project,
        })
    }
}
