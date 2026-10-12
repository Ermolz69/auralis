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
}

impl<R: ProjectRepository> CreateProjectUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self {
            project_repo,
            contract: None,
            required_contract: None,
        }
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

        Ok(CreateProjectResponse {
            project: created_project,
        })
    }
}
