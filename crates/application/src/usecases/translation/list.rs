use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::repository::ProjectRepository;
use ports::translation_links::{TranslationLink, TranslationLinkStore};
use std::sync::Arc;

pub struct ListProjectTranslationsUseCase {
    projects: Arc<dyn ProjectRepository>,
    links: Arc<dyn TranslationLinkStore>,
}

impl ListProjectTranslationsUseCase {
    pub fn new(projects: Arc<dyn ProjectRepository>, links: Arc<dyn TranslationLinkStore>) -> Self {
        Self { projects, links }
    }

    pub async fn execute(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<TranslationLink>, ApplicationError> {
        self.projects
            .get(project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project_id.clone()))?;
        Ok(self.links.list_by_project(project_id).await?)
    }
}
