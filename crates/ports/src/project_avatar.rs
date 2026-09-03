use crate::error::PortError;
use async_trait::async_trait;
use domain::project::{ProjectId, avatar::ProjectAvatar};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAvatarRecord {
    pub avatar: Option<ProjectAvatar>,
    pub initialized: bool,
}

#[async_trait]
pub trait ProjectAvatarRepository: Send + Sync {
    async fn get(&self, project_id: &ProjectId) -> Result<ProjectAvatarRecord, PortError>;
    async fn set(
        &self,
        project_id: &ProjectId,
        avatar: Option<ProjectAvatar>,
        only_if_missing: bool,
    ) -> Result<ProjectAvatarRecord, PortError>;
}
