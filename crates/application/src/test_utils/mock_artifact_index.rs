use async_trait::async_trait;
use domain::media::{Artifact, ArtifactId, ArtifactState};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MockArtifactIndex {
    pub artifacts: Arc<Mutex<Vec<Artifact>>>,
}

impl MockArtifactIndex {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ArtifactIndex for MockArtifactIndex {
    async fn check_exists(&self, id: &ArtifactId) -> Result<bool, PortError> {
        let lock = self.artifacts.lock().await;
        Ok(lock.iter().any(|a| a.id == *id))
    }

    async fn add(&self, _project_id: &ProjectId, artifact: &Artifact) -> Result<(), PortError> {
        let mut lock = self.artifacts.lock().await;
        lock.push(artifact.clone());
        Ok(())
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
        let lock = self.artifacts.lock().await;
        Ok(lock.iter().find(|a| a.id == *id).cloned())
    }

    async fn update_state(
        &self,
        id: &ArtifactId,
        state: ArtifactState,
        _time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), PortError> {
        let mut lock = self.artifacts.lock().await;
        if let Some(artifact) = lock.iter_mut().find(|a| a.id == *id) {
            artifact.state = state;
        }
        Ok(())
    }

    async fn list_by_project(&self, _project_id: &ProjectId) -> Result<Vec<Artifact>, PortError> {
        Ok(vec![])
    }

    async fn list_by_project_and_kind(
        &self,
        _project_id: &ProjectId,
        kind: domain::media::ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError> {
        let lock = self.artifacts.lock().await;
        Ok(lock.iter().filter(|a| a.kind == kind).cloned().collect())
    }

    async fn delete(&self, id: &ArtifactId) -> Result<(), PortError> {
        let mut lock = self.artifacts.lock().await;
        lock.retain(|a| a.id != *id);
        Ok(())
    }
}
