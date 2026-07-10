use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::{JobRepository, ProjectRepository};

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

impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
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

impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
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
        Ok(lock
            .values()
            .filter(|j| j.project_id() == project_id)
            .cloned()
            .collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        Ok(lock
            .values()
            .filter(|j| j.status() == &domain::job::JobStatus::Running)
            .cloned()
            .collect())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        let mut jobs: Vec<Job> = lock.values().cloned().collect();
        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        Ok(jobs.into_iter().take(limit).collect())
    }
}

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
        Ok(lock
            .iter()
            .find(|(_, a)| &a.id == id)
            .map(|(_, a)| a.clone()))
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





use ports::transaction::{StorageUnitOfWork, CommitTranscriptImport, CommitMediaDownload, CommitProjectDelete, CommitJobUpdate};

#[derive(Clone)]
pub struct InMemoryStorageUnitOfWork {
    project_repo: Arc<InMemoryProjectRepository>,
    job_repo: Arc<InMemoryJobRepository>,
}

impl InMemoryStorageUnitOfWork {
    pub fn new(
        project_repo: Arc<InMemoryProjectRepository>,
        job_repo: Arc<InMemoryJobRepository>,
    ) -> Self {
        Self {
            project_repo,
            job_repo,
        }
    }
}

#[async_trait]
impl StorageUnitOfWork for InMemoryStorageUnitOfWork {
    async fn commit_transcript_import(&self, command: CommitTranscriptImport) -> Result<(), PortError> {
        self.project_repo.save(&command.project).await?;
        Ok(())
    }

    async fn commit_media_download(&self, command: CommitMediaDownload) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        self.project_repo.delete(&command.project_id).await?;
        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        self.job_repo.save(&command.job).await?;
        Ok(())
    }
}