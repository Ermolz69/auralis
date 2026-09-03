use crate::error::ApplicationError;
use domain::project::{ProjectId, avatar::ProjectAvatar};
use ports::project_avatar::{ProjectAvatarRecord, ProjectAvatarRepository};
use std::sync::Arc;

pub struct ProjectAvatarUseCase {
    repository: Arc<dyn ProjectAvatarRepository>,
}

impl ProjectAvatarUseCase {
    pub fn new(repository: Arc<dyn ProjectAvatarRepository>) -> Self {
        Self { repository }
    }
    pub async fn get(
        &self,
        project_id: ProjectId,
    ) -> Result<ProjectAvatarRecord, ApplicationError> {
        Ok(self.repository.get(&project_id).await?)
    }
    pub async fn set(
        &self,
        project_id: ProjectId,
        data_url: Option<String>,
        only_if_missing: bool,
    ) -> Result<ProjectAvatarRecord, ApplicationError> {
        let avatar = data_url.map(ProjectAvatar::new).transpose()?;
        Ok(self
            .repository
            .set(&project_id, avatar, only_if_missing)
            .await?)
    }
}
