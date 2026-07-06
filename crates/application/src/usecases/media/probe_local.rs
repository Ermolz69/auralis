use std::path::PathBuf;

use domain::media::{MediaMetadata, MediaSource};
use domain::project::{Project, ProjectId};
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct ProbeLocalMediaRequest {
    pub project_id: Option<ProjectId>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ProbeLocalMediaResponse {
    pub metadata: MediaMetadata,
    pub project: Option<Project>,
}

pub struct ProbeLocalMediaUseCase<R: ProjectRepository, P: MediaProbePort> {
    project_repo: R,
    media_probe: P,
}

impl<R: ProjectRepository, P: MediaProbePort> ProbeLocalMediaUseCase<R, P> {
    pub fn new(project_repo: R, media_probe: P) -> Self {
        Self {
            project_repo,
            media_probe,
        }
    }

    pub async fn execute(
        &self,
        request: ProbeLocalMediaRequest,
    ) -> Result<ProbeLocalMediaResponse, ApplicationError> {
        if !request.path.exists() {
            return Err(ApplicationError::InvalidOperation {
                message: format!("Path does not exist: {:?}", request.path),
            });
        }

        let metadata = self.media_probe.probe_local_file(&request.path).await?;

        let mut project_out = None;

        if let Some(pid) = request.project_id {
            let mut project = self
                .project_repo
                .get(&pid)
                .await?
                .ok_or_else(|| ApplicationError::ProjectNotFound(pid.clone()))?;

            let source = MediaSource::LocalFile {
                path: request.path.to_string_lossy().to_string(),
            };

            project.import_source(source, Some(metadata.clone()))?;
            self.project_repo.save(&project).await?;

            project_out = Some(project);
        }

        Ok(ProbeLocalMediaResponse {
            metadata,
            project: project_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_ffmpeg::mock::MockMediaProbeAdapter;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::project::ProjectStatus;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_probe_local_media_success_with_project() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let use_case = ProbeLocalMediaUseCase::new(repo.clone(), probe);

        let project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        repo.create(project.clone()).await.unwrap();

        // Create a dummy temp file to pass exists() check
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mp4");
        File::create(&file_path).unwrap();

        let request = ProbeLocalMediaRequest {
            project_id: Some(project_id.clone()),
            path: file_path,
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.metadata.duration_ms, 5000);
        let updated_project = response.project.unwrap();
        assert_eq!(*updated_project.status(), ProjectStatus::SourceImported);
        assert!(updated_project.source().is_some());
        assert!(updated_project.metadata().is_some());

        let saved = repo.get(&project_id).await.unwrap().unwrap();
        assert_eq!(*saved.status(), ProjectStatus::SourceImported);
    }

    #[tokio::test]
    async fn test_probe_local_media_without_project() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let use_case = ProbeLocalMediaUseCase::new(repo.clone(), probe);

        // Create a dummy temp file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mkv");
        File::create(&file_path).unwrap();

        let request = ProbeLocalMediaRequest {
            project_id: None,
            path: file_path,
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.metadata.duration_ms, 5000);
        assert!(response.project.is_none());
    }

    #[tokio::test]
    async fn test_probe_local_media_not_found() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let use_case = ProbeLocalMediaUseCase::new(repo, probe);

        let request = ProbeLocalMediaRequest {
            project_id: None,
            path: PathBuf::from("/non/existent/path.mp4"),
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
    }
}
