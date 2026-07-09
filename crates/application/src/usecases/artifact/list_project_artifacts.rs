use crate::error::ApplicationError;
use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;

pub struct ListProjectArtifactsRequest {
    pub project_id: ProjectId,
    pub kind: Option<ArtifactKind>,
}

pub struct ListProjectArtifactsUseCase<I>
where
    I: ArtifactIndex,
{
    artifact_index: I,
}

impl<I> ListProjectArtifactsUseCase<I>
where
    I: ArtifactIndex,
{
    pub fn new(artifact_index: I) -> Self {
        Self { artifact_index }
    }

    pub async fn execute(
        &self,
        request: ListProjectArtifactsRequest,
    ) -> Result<Vec<Artifact>, ApplicationError> {
        let artifacts = if let Some(kind) = request.kind {
            self.artifact_index
                .list_by_project_and_kind(&request.project_id, kind)
                .await?
        } else {
            self.artifact_index
                .list_by_project(&request.project_id)
                .await?
        };

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::media::{ArtifactId, ArtifactLocation};
    use ports::error::PortError;

    #[derive(Clone)]
    struct MockArtifactIndex {
        artifacts: Vec<Artifact>,
    }

    #[async_trait]
    impl ArtifactIndex for MockArtifactIndex {
        async fn add(
            &self,
            _project_id: &ProjectId,
            _artifact: &Artifact,
        ) -> Result<(), PortError> {
            Ok(())
        }

        async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
            Ok(self.artifacts.iter().find(|a| &a.id == id).cloned())
        }

        async fn list_by_project(
            &self,
            _project_id: &ProjectId,
        ) -> Result<Vec<Artifact>, PortError> {
            Ok(self.artifacts.clone())
        }

        async fn list_by_project_and_kind(
            &self,
            _project_id: &ProjectId,
            kind: ArtifactKind,
        ) -> Result<Vec<Artifact>, PortError> {
            Ok(self
                .artifacts
                .iter()
                .filter(|a| a.kind == kind)
                .cloned()
                .collect())
        }

        async fn delete(&self, _id: &domain::media::ArtifactId) -> Result<(), PortError> {
            Ok(())
        }
        async fn update_state(
            &self,
            _id: &domain::media::ArtifactId,
            _state: domain::media::ArtifactState,
            _ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
        ) -> Result<(), PortError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_list_project_artifacts_all() {
        let artifact1 = Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::ExtractedAudio,
            location: ArtifactLocation::LocalPath("/test.wav".into()),
            size_bytes: Some(100),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        };
        let artifact2 = Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::LocalPath("/test.vtt".into()),
            size_bytes: Some(100),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        };

        let index = MockArtifactIndex {
            artifacts: vec![artifact1.clone(), artifact2.clone()],
        };

        let use_case = ListProjectArtifactsUseCase::new(index);
        let result = use_case
            .execute(ListProjectArtifactsRequest {
                project_id: ProjectId::new(),
                kind: None,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_list_project_artifacts_by_kind() {
        let artifact1 = Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::ExtractedAudio,
            location: ArtifactLocation::LocalPath("/test.wav".into()),
            size_bytes: Some(100),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        };
        let artifact2 = Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::LocalPath("/test.vtt".into()),
            size_bytes: Some(100),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        };

        let index = MockArtifactIndex {
            artifacts: vec![artifact1.clone(), artifact2.clone()],
        };

        let use_case = ListProjectArtifactsUseCase::new(index);
        let result = use_case
            .execute(ListProjectArtifactsRequest {
                project_id: ProjectId::new(),
                kind: Some(ArtifactKind::OriginalSubtitle),
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, artifact2.id);
    }
}
