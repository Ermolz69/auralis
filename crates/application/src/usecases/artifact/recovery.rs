use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::artifact_recovery::ArtifactRecoveryRepository;
use std::sync::Arc;

pub struct ArtifactRecoveryUseCase {
    repository: Arc<dyn ArtifactRecoveryRepository>,
}

impl ArtifactRecoveryUseCase {
    pub fn new(repository: Arc<dyn ArtifactRecoveryRepository>) -> Self {
        Self { repository }
    }
    pub async fn list(&self) -> Result<Vec<ProjectId>, ApplicationError> {
        Ok(self.repository.list_projects().await?)
    }
    pub async fn retry(&self, project_id: &ProjectId) -> Result<(), ApplicationError> {
        Ok(self.repository.retry_finalization(project_id).await?)
    }
}
