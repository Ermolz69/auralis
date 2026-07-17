#![allow(clippy::unwrap_used, clippy::expect_used)]
use domain::media::MediaSource;
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use ports::source::VideoSourcePort;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct ImportVideoSourceRequest {
    pub project_id: ProjectId,
    pub source: MediaSource,
}

#[derive(Debug)]
pub struct ImportVideoSourceResponse {
    pub project: Project,
}

pub struct ImportVideoSourceUseCase<R: ProjectRepository, V: VideoSourcePort> {
    project_repo: R,
    video_source: V,
}

impl<R: ProjectRepository, V: VideoSourcePort> ImportVideoSourceUseCase<R, V> {
    pub fn new(project_repo: R, video_source: V) -> Self {
        Self {
            project_repo,
            video_source,
        }
    }

    pub async fn execute(
        &self,
        request: ImportVideoSourceRequest,
    ) -> Result<ImportVideoSourceResponse, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        self.video_source.validate_source(&request.source).await?;
        let metadata = self.video_source.fetch_metadata(&request.source).await?;

        project.import_source(request.source, Some(metadata))?;
        self.project_repo.save(&project).await?;

        Ok(ImportVideoSourceResponse { project })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::InMemoryProjectRepository;
    use adapters_ytdlp::mock::MockVideoSourceAdapter;
    use domain::project::{Project, ProjectStatus};

    #[tokio::test]
    async fn test_import_video_source_success() {
        let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        // Create project first
        let project = Project::new("Test Project".to_string());
        let project_id = project.id().clone();
        repo.create(project).await.unwrap();

        let source_port = MockVideoSourceAdapter::new();
        let use_case = ImportVideoSourceUseCase::new(repo.clone(), source_port);

        let request = ImportVideoSourceRequest {
            project_id: project_id.clone(),
            source: MediaSource::RemoteUrl {
                url: "https://youtube.com/watch?v=123".to_string(),
            },
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.project.id(), &project_id);

        // Verify it was updated in repo
        let updated = repo.get(&project_id).await.unwrap().unwrap();
        assert!(updated.source().is_some());
    }

    #[tokio::test]
    async fn test_import_source_not_found() {
        let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        let port = MockVideoSourceAdapter::new();
        let use_case = ImportVideoSourceUseCase::new(repo, port);

        let request = ImportVideoSourceRequest {
            project_id: domain::project::ProjectId::new(),
            source: MediaSource::RemoteUrl {
                url: "http://example.com/video.mp4".to_string(),
            },
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::ProjectNotFound(_)));
    }

    #[tokio::test]
    async fn test_import_source_validation_fails() {
        let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        let port = MockVideoSourceAdapter::failing();

        let project = Project::new("Test".to_string());
        repo.create(project.clone()).await.unwrap();

        let use_case = ImportVideoSourceUseCase::new(repo.clone(), port);

        let request = ImportVideoSourceRequest {
            project_id: project.id().clone(),
            source: MediaSource::RemoteUrl {
                url: "http://example.com/video.mp4".to_string(),
            },
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Port(_)));

        // Ensure not saved
        let saved = repo.get(project.id()).await.unwrap().unwrap();
        assert_eq!(*saved.status(), ProjectStatus::Draft);
    }
}
