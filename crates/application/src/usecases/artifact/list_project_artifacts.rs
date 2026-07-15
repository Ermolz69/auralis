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
    use domain::media::{ArtifactId, ArtifactLocation};

    use crate::test_utils::MockArtifactIndex;
    use std::sync::Arc;
    use tokio::sync::Mutex;

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

        let project_id = ProjectId::new();
        let index = MockArtifactIndex {
            artifacts: Arc::new(Mutex::new(vec![
                (project_id.clone(), artifact1.clone()),
                (project_id.clone(), artifact2.clone()),
            ])),
        };

        let use_case = ListProjectArtifactsUseCase::new(index);
        let result = use_case
            .execute(ListProjectArtifactsRequest {
                project_id,
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

        let project_id = ProjectId::new();
        let index = MockArtifactIndex {
            artifacts: Arc::new(Mutex::new(vec![
                (project_id.clone(), artifact1.clone()),
                (project_id.clone(), artifact2.clone()),
            ])),
        };

        let use_case = ListProjectArtifactsUseCase::new(index);
        let result = use_case
            .execute(ListProjectArtifactsRequest {
                project_id,
                kind: Some(ArtifactKind::OriginalSubtitle),
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, artifact2.id);
    }
}
