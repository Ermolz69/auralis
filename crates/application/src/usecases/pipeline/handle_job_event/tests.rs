use super::*;
use async_trait::async_trait;
use domain::job::JobId;
use domain::job::JobProgress;
use domain::media::SubtitleTrack;
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MockSubtitleSource;

#[async_trait]
impl SubtitleSourcePort for MockSubtitleSource {
    async fn list_subtitles(
        &self,
        _source: &domain::media::MediaSource,
    ) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(vec![])
    }

    async fn download_subtitle(
        &self,
        _source: &domain::media::MediaSource,
        _track: &SubtitleTrack,
        _target_path: &std::path::Path,
    ) -> Result<domain::media::Artifact, PortError> {
        Err(PortError::Unsupported {
            message: "Not implemented".into(),
        })
    }
}

#[derive(Clone)]
struct MockProjectRepo {
    project: Arc<Mutex<Option<Project>>>,
}

impl MockProjectRepo {
    fn new(project: Project) -> Self {
        Self {
            project: Arc::new(Mutex::new(Some(project))),
        }
    }
}

#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
        let guard = self.project.lock().unwrap();
        Ok(guard.clone())
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        let mut guard = self.project.lock().unwrap();
        *guard = Some(project.clone());
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

#[derive(Clone, Default)]
struct MockAppEventPublisher {
    events: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl AppEventPublisher for MockAppEventPublisher {
    async fn publish_project_updated(&self, project_id: &str) -> Result<(), PortError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("project_updated:{}", project_id));
        Ok(())
    }

    async fn publish_transcript_ready(
        &self,
        project_id: &str,
        job_id: &str,
    ) -> Result<(), PortError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("transcript_ready:{}:{}", project_id, job_id));
        Ok(())
    }
}

#[derive(Clone)]
struct MockArtifactIndex;

#[async_trait]
impl ports::artifact_index::ArtifactIndex for MockArtifactIndex {
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
}

#[derive(Clone)]
struct MockArtifactStore;

#[async_trait]
impl ports::storage::ArtifactStore for MockArtifactStore {
    async fn project_dir(&self, _project_id: &ProjectId) -> Result<std::path::PathBuf, PortError> {
        Ok(std::path::PathBuf::from("/tmp"))
    }

    async fn reserve_artifact_path(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _extension: &str,
    ) -> Result<std::path::PathBuf, PortError> {
        Ok(std::path::PathBuf::from("/tmp/artifact"))
    }

    async fn register_artifact(
        &self,
        _project_id: &ProjectId,
        _artifact: &domain::media::Artifact,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_artifact(
        &self,
        _artifact: &domain::media::Artifact,
    ) -> Result<std::path::PathBuf, PortError> {
        Ok(std::path::PathBuf::from("/tmp/artifact"))
    }

    async fn write_small_artifact(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _filename: &str,
        _data: &[u8],
    ) -> Result<domain::media::Artifact, PortError> {
        Ok(domain::media::Artifact {
            id: domain::media::ArtifactId::new(),
            kind: domain::media::ArtifactKind::OriginalSubtitle,
            location: domain::media::ArtifactLocation::StorageKey("test".to_string()),
            size_bytes: Some(10),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }

    async fn import_artifact(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<domain::media::Artifact, PortError> {
        Ok(domain::media::Artifact {
            id: domain::media::ArtifactId::new(),
            kind: domain::media::ArtifactKind::DownloadedVideo,
            location: domain::media::ArtifactLocation::StorageKey("test_video.mp4".to_string()),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }

    async fn delete_artifact(&self, _artifact: &domain::media::Artifact) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
}

fn create_processing_project() -> Project {
    let mut p = Project::new("Test".into());
    p.import_source(
        domain::media::MediaSource::LocalFile { path: "".into() },
        None,
    )
    .unwrap();
    p.mark_ready_for_processing().unwrap();
    p.mark_processing_started().unwrap();
    p
}

fn create_processing_youtube_project() -> Project {
    let mut p = Project::new("YT".into());
    p.import_source(
        domain::media::MediaSource::YoutubeUrl { url: "".into() },
        None,
    )
    .unwrap();
    p.mark_ready_for_processing().unwrap();
    p.mark_processing_started().unwrap();
    p
}

#[tokio::test]
async fn test_queued_running_noop() {
    let repo = MockProjectRepo::new(create_processing_project());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo,
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    for status in [JobStatus::Running, JobStatus::Pending] {
        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(ProjectId::new()),
            status,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.execute(event).await.unwrap();
    }

    assert!(publ.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_no_project_id_noop() {
    let repo = MockProjectRepo::new(create_processing_project());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo,
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    let event = JobLifecycleEvent {
        job_id: JobId::new(),
        project_id: None,
        status: JobStatus::Completed,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
    };
    uc.execute(event).await.unwrap();

    assert!(publ.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_completed_local_media() {
    let p = create_processing_project();
    let pid_str = p.id().to_string();
    let repo = MockProjectRepo::new(p.clone());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo.clone(),
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    let event = JobLifecycleEvent {
        job_id: JobId::new(),
        project_id: Some(p.id().clone()),
        status: JobStatus::Completed,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
    };
    uc.execute(event).await.unwrap();

    let events = publ.events.lock().unwrap().clone();
    assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);

    let p2 = repo
        .get(&domain::project::ProjectId::from_str(&pid_str).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p2.status(), &domain::project::ProjectStatus::Completed);
}

#[tokio::test]
async fn test_completed_youtube_subtitles_fail() {
    let p = create_processing_youtube_project();
    let pid_str = p.id().to_string();
    let repo = MockProjectRepo::new(p.clone());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo.clone(),
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    let event = JobLifecycleEvent {
        job_id: JobId::new(),
        project_id: Some(p.id().clone()),
        status: JobStatus::Completed,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
    };
    uc.execute(event).await.unwrap();

    let events = publ.events.lock().unwrap().clone();
    assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);

    let p2 = repo
        .get(&domain::project::ProjectId::from_str(&pid_str).unwrap())
        .await
        .unwrap()
        .unwrap();
    // Failed because MockSubtitleSource returns err
    assert_eq!(p2.status(), &domain::project::ProjectStatus::Failed);
}

#[tokio::test]
async fn test_failed() {
    let p = create_processing_project();
    let pid_str = p.id().to_string();
    let repo = MockProjectRepo::new(p.clone());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo.clone(),
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    let event = JobLifecycleEvent {
        job_id: JobId::new(),
        project_id: Some(p.id().clone()),
        status: JobStatus::Failed,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
    };
    uc.execute(event).await.unwrap();

    let events = publ.events.lock().unwrap().clone();
    assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);

    let p2 = repo
        .get(&domain::project::ProjectId::from_str(&pid_str).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p2.status(), &domain::project::ProjectStatus::Failed);
}

#[tokio::test]
async fn test_cancelled() {
    let p = create_processing_project();
    let pid_str = p.id().to_string();
    let repo = MockProjectRepo::new(p.clone());
    let publ = MockAppEventPublisher::default();
    let uc = HandleJobEventUseCase::new(
        repo.clone(),
        MockSubtitleSource,
        publ.clone(),
        MockArtifactIndex,
        MockArtifactStore,
    );

    let event = JobLifecycleEvent {
        job_id: JobId::new(),
        project_id: Some(p.id().clone()),
        status: JobStatus::Cancelled,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
    };
    uc.execute(event).await.unwrap();

    let events = publ.events.lock().unwrap().clone();
    assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);

    let p2 = repo
        .get(&domain::project::ProjectId::from_str(&pid_str).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p2.status(), &domain::project::ProjectStatus::Cancelled);
}
