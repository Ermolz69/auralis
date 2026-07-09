use crate::error::ApplicationError;
use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use std::str::FromStr;
use std::sync::Arc;

pub struct HandleJobCompletedRequest {
    pub job_id: String,
    pub project_id: String,
    pub is_success: bool,
    pub target_dir_base: std::path::PathBuf,
}

pub struct HandleJobCompletedResult {
    pub transcript_ready: bool,
}

pub struct HandleJobCompletedUseCase<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    I: ArtifactIndex + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    video_source: V,
    artifact_index: I,
    artifact_store: S,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    I: ArtifactIndex + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> HandleJobCompletedUseCase<R, V, I, S>
{
    pub fn new(project_repo: R, video_source: V, artifact_index: I, artifact_store: S) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_index,
            artifact_store,
        }
    }

    pub async fn execute(
        &self,
        req: HandleJobCompletedRequest,
    ) -> Result<HandleJobCompletedResult, ApplicationError> {
        let pid = domain::project::ProjectId::from_str(&req.project_id).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: e.to_string(),
            }
        })?;

        let mut project = self
            .project_repo
            .get(&pid)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(pid.clone()))?;

        let mut transcript_ready = false;

        if req.is_success {
            let is_youtube = matches!(
                project.source(),
                Some(domain::media::MediaSource::YoutubeUrl { .. })
            );

            if is_youtube {
                let target_dir = req
                    .target_dir_base
                    .join("auralis")
                    .join("projects")
                    .join(&req.project_id)
                    .join("subtitles");

                let import_use_case = ImportYoutubeSubtitlesUseCase::new(
                    Arc::new(self.project_repo.clone()),
                    Arc::new(self.video_source.clone()),
                    Arc::new(self.artifact_index.clone()),
                    Arc::new(self.artifact_store.clone()),
                );

                match import_use_case
                    .execute(ImportYoutubeSubtitlesRequest {
                        project_id: pid.clone(),
                        target_dir,
                        preferred_languages: vec![
                            "en".to_string(),
                            "ru".to_string(),
                            "uk".to_string(),
                        ],
                        allow_auto_generated: true,
                    })
                    .await
                {
                    Ok(_) => {
                        // Re-fetch project to ensure we have the updated version with transcript
                        if let Ok(Some(updated_project)) = self.project_repo.get(&pid).await {
                            project = updated_project;
                        }
                        transcript_ready = true;
                        project.mark_completed()?;
                    }
                    Err(_) => {
                        project.mark_failed()?;
                    }
                }
            } else {
                project.mark_completed()?;
            }
        } else {
            project.mark_failed()?;
        }

        self.project_repo.save(&project).await?;

        Ok(HandleJobCompletedResult { transcript_ready })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::project::{Project, ProjectId};
    use ports::error::PortError;
    use std::path::PathBuf;

    #[derive(Clone)]
    struct MockSubtitleSource;

    #[async_trait]
    impl SubtitleSourcePort for MockSubtitleSource {
        async fn list_subtitles(
            &self,
            _source: &domain::media::MediaSource,
        ) -> Result<Vec<domain::media::SubtitleTrack>, PortError> {
            Ok(vec![])
        }

        async fn download_subtitle(
            &self,
            _source: &domain::media::MediaSource,
            _track: &domain::media::SubtitleTrack,
            _target_path: &std::path::Path,
        ) -> Result<domain::media::Artifact, PortError> {
            Err(PortError::Unsupported {
                message: "Not implemented".into(),
            })
        }
    }

    #[derive(Clone)]
    struct FailingSaveRepo;

    #[async_trait]
    impl ProjectRepository for FailingSaveRepo {
        async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
            let mut project = Project::new("Test".into());
            // Need processing state to pass the transition check
            project
                .import_source(
                    domain::media::MediaSource::LocalFile { path: "".into() },
                    None,
                )
                .unwrap();
            project.mark_ready_for_processing().unwrap();
            project.mark_processing_started().unwrap();
            Ok(Some(project))
        }

        async fn save(&self, _project: &Project) -> Result<(), PortError> {
            Err(PortError::Io {
                message: "Save failed".into(),
            })
        }

        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            unimplemented!()
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            unimplemented!()
        }
    }

    #[derive(Clone)]
    struct DraftProjectRepo;

    #[async_trait]
    impl ProjectRepository for DraftProjectRepo {
        async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
            // Project is in Draft state, which will fail mark_completed()
            Ok(Some(Project::new("Draft".into())))
        }

        async fn save(&self, _project: &Project) -> Result<(), PortError> {
            Ok(())
        }

        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            unimplemented!()
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            unimplemented!()
        }
    }

    #[derive(Clone)]
    struct MockArtifactIndex;

    #[async_trait]
    impl ArtifactIndex for MockArtifactIndex {
        async fn add(
            &self,
            _project_id: &ProjectId,
            _artifact: &domain::media::Artifact,
        ) -> Result<(), PortError> {
            Ok(())
        }
        async fn get(
            &self,
            _id: &domain::media::ArtifactId,
        ) -> Result<Option<domain::media::Artifact>, PortError> {
            Ok(None)
        }
        async fn list_by_project(
            &self,
            _project_id: &ProjectId,
        ) -> Result<Vec<domain::media::Artifact>, PortError> {
            Ok(vec![])
        }
        async fn list_by_project_and_kind(
            &self,
            _project_id: &ProjectId,
            _kind: domain::media::ArtifactKind,
        ) -> Result<Vec<domain::media::Artifact>, PortError> {
            Ok(vec![])
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

    use crate::test_utils::MockArtifactStore;

    #[tokio::test]
    async fn test_transition_failure_propagates() {
        let repo = DraftProjectRepo;
        let use_case = HandleJobCompletedUseCase::new(
            repo,
            MockSubtitleSource,
            MockArtifactIndex,
            MockArtifactStore,
        );

        let req = HandleJobCompletedRequest {
            job_id: "job-1".into(),
            project_id: ProjectId::new().to_string(),
            is_success: true,
            target_dir_base: PathBuf::from("/tmp"),
        };

        let result = use_case.execute(req).await;
        assert!(
            result.is_err(),
            "Expected transition to fail for Draft project"
        );
        if let Err(ApplicationError::Domain(e)) = result {
            match e {
                domain::error::DomainError::InvalidStateTransition { .. } => {}
                _ => panic!("Expected InvalidStateTransition error"),
            }
        } else {
            panic!("Expected Domain error for transition failure");
        }
    }

    #[tokio::test]
    async fn test_save_failure_propagates() {
        let repo = FailingSaveRepo;
        let use_case = HandleJobCompletedUseCase::new(
            repo,
            MockSubtitleSource,
            MockArtifactIndex,
            MockArtifactStore,
        );

        let req = HandleJobCompletedRequest {
            job_id: "job-1".into(),
            project_id: ProjectId::new().to_string(),
            is_success: true,
            target_dir_base: PathBuf::from("/tmp"),
        };

        let result = use_case.execute(req).await;
        assert!(result.is_err(), "Expected save to fail");
        if let Err(ApplicationError::Port(e)) = result {
            match e {
                PortError::Io { .. } => {}
                _ => panic!("Expected Io Error"),
            }
        } else {
            panic!("Expected Port error for save failure");
        }
    }
}
