#[cfg(test)]
pub mod mocks {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;
    use domain::job::{Job, JobId};
    use domain::media::{Artifact, MediaMetadata, MediaSource};
    use domain::project::{Project, ProjectId};
    use ports::error::PortError;
    use ports::repository::{JobRepository, ProjectRepository};
    use ports::source::{DownloadMediaRequest, VideoSourcePort};

    #[derive(Clone)]
    pub struct InMemoryProjectRepository {
        pub projects: Arc<Mutex<HashMap<ProjectId, Project>>>,
    }

    impl InMemoryProjectRepository {
        pub fn new() -> Self {
            Self {
                projects: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl ProjectRepository for InMemoryProjectRepository {
        async fn create(&self, project: Project) -> Result<Project, PortError> {
            let mut lock = self.projects.lock().unwrap();
            lock.insert(project.id().clone(), project.clone());
            Ok(project)
        }

        async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
            let lock = self.projects.lock().unwrap();
            Ok(lock.get(id).cloned())
        }

        async fn save(&self, project: &Project) -> Result<(), PortError> {
            let mut lock = self.projects.lock().unwrap();
            lock.insert(project.id().clone(), project.clone());
            Ok(())
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            let lock = self.projects.lock().unwrap();
            Ok(lock.values().cloned().collect())
        }

        async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
            let mut lock = self.projects.lock().unwrap();
            lock.remove(id);
            Ok(())
        }
    }

    #[derive(Clone)]
    pub struct InMemoryJobRepository {
        pub jobs: Arc<Mutex<HashMap<JobId, Job>>>,
    }

    impl InMemoryJobRepository {
        pub fn new() -> Self {
            Self {
                jobs: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl JobRepository for InMemoryJobRepository {
        async fn create(&self, job: Job) -> Result<Job, PortError> {
            let mut lock = self.jobs.lock().unwrap();
            lock.insert(job.id().clone(), job.clone());
            Ok(job)
        }

        async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
            let lock = self.jobs.lock().unwrap();
            Ok(lock.get(id).cloned())
        }

        async fn save(&self, job: &Job) -> Result<(), PortError> {
            let mut lock = self.jobs.lock().unwrap();
            lock.insert(job.id().clone(), job.clone());
            Ok(())
        }

        async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
            let lock = self.jobs.lock().unwrap();
            Ok(lock.values().filter(|j| j.project_id() == project_id).cloned().collect())
        }

        async fn list_active(&self) -> Result<Vec<Job>, PortError> {
            let lock = self.jobs.lock().unwrap();
            Ok(lock.values().filter(|j| j.status() == &domain::job::JobStatus::Running).cloned().collect())
        }
    }

    pub struct MockVideoSourcePort {
        pub should_fail_validation: bool,
    }

    impl MockVideoSourcePort {
        pub fn new() -> Self {
            Self {
                should_fail_validation: false,
            }
        }
        
        pub fn failing() -> Self {
            Self {
                should_fail_validation: true,
            }
        }
    }

    #[async_trait]
    impl VideoSourcePort for MockVideoSourcePort {
        async fn validate_source(&self, _source: &MediaSource) -> Result<(), PortError> {
            if self.should_fail_validation {
                return Err(PortError::InvalidSource { message: "Validation failed".to_string() });
            }
            Ok(())
        }

        async fn fetch_metadata(&self, _source: &MediaSource) -> Result<MediaMetadata, PortError> {
            Ok(MediaMetadata {
                duration_ms: 1000,
                width: Some(1920),
                height: Some(1080),
                fps: Some(60.0),
                video_codec: Some("h264".to_string()),
                audio_codec: Some("aac".to_string()),
                audio_channels: Some(2),
                sample_rate: Some(48000),
                container: Some("mp4".to_string()),
                has_video: true,
                has_audio: true,
            })
        }

        async fn download_media(&self, _request: DownloadMediaRequest) -> Result<Artifact, PortError> {
            unimplemented!()
        }
    }
}
