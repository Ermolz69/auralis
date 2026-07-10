use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use domain::project::ProjectId;
use ports::error::PortError;

#[derive(Clone)]
pub struct InMemoryArtifactIndex {
    pub artifacts: Arc<Mutex<Vec<(ProjectId, domain::media::Artifact)>>>,
}

impl InMemoryArtifactIndex {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryArtifactIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ports::artifact_index::ArtifactIndex for InMemoryArtifactIndex {
    async fn add(
        &self,
        project_id: &ProjectId,
        artifact: &domain::media::Artifact,
    ) -> Result<(), PortError> {
        let mut lock = self.artifacts.lock().unwrap();
        // Remove existing with same ID if any
        lock.retain(|(_, a)| a.id != artifact.id);
        lock.push((project_id.clone(), artifact.clone()));
        Ok(())
    }

    async fn get(
        &self,
        id: &domain::media::ArtifactId,
    ) -> Result<Option<domain::media::Artifact>, PortError> {
        let lock = self.artifacts.lock().unwrap();
        let artifact = lock
            .iter()
            .find(|(_, a)| a.id == *id && a.state == domain::media::ArtifactState::Ready)
            .map(|(_, a)| a.clone());
        Ok(artifact)
    }

    async fn check_exists(&self, id: &domain::media::ArtifactId) -> Result<bool, PortError> {
        let lock = self.artifacts.lock().unwrap();
        Ok(lock.iter().any(|(_, a)| &a.id == id))
    }

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<domain::media::Artifact>, PortError> {
        let lock = self.artifacts.lock().unwrap();
        Ok(lock
            .iter()
            .filter(|(pid, _)| pid == project_id)
            .map(|(_, a)| a.clone())
            .collect())
    }

    async fn list_by_project_and_kind(
        &self,
        project_id: &ProjectId,
        kind: domain::media::ArtifactKind,
    ) -> Result<Vec<domain::media::Artifact>, PortError> {
        let lock = self.artifacts.lock().unwrap();
        Ok(lock
            .iter()
            .filter(|(pid, a)| pid == project_id && a.kind == kind)
            .map(|(_, a)| a.clone())
            .collect())
    }

    async fn delete(&self, id: &domain::media::ArtifactId) -> Result<(), PortError> {
        let mut artifacts = self.artifacts.lock().unwrap();
        artifacts.retain(|(_, a)| a.id != *id);
        Ok(())
    }

    async fn update_state(
        &self,
        id: &domain::media::ArtifactId,
        state: domain::media::ArtifactState,
        ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
    ) -> Result<(), PortError> {
        let mut artifacts = self.artifacts.lock().unwrap();
        if let Some((_, artifact)) = artifacts.iter_mut().find(|(_, a)| a.id == *id) {
            artifact.state = state;
            if let Some(r) = ready_at {
                artifact.ready_at = Some(r);
            }
            artifact.updated_at = domain::chrono::Utc::now();
        }
        Ok(())
    }
}
