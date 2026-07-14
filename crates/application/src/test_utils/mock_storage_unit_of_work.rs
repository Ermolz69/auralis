use async_trait::async_trait;
use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitPipelineStart, CommitPipelineStartFailure, CommitProjectDelete,
    CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockStorageUnitOfWork {
    pub jobs_saved: Arc<Mutex<Vec<domain::job::Job>>>,
    pub projects_saved: Arc<Mutex<Vec<domain::project::Project>>>,
    pub projects_deleted: Arc<Mutex<Vec<domain::project::ProjectId>>>,
    pub should_fail: bool,
}

impl Default for MockStorageUnitOfWork {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStorageUnitOfWork {
    pub fn new() -> Self {
        Self {
            jobs_saved: Arc::new(Mutex::new(Vec::new())),
            projects_saved: Arc::new(Mutex::new(Vec::new())),
            projects_deleted: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
        }
    }

    pub fn with_failure() -> Self {
        Self {
            jobs_saved: Arc::new(Mutex::new(Vec::new())),
            projects_saved: Arc::new(Mutex::new(Vec::new())),
            projects_deleted: Arc::new(Mutex::new(Vec::new())),
            should_fail: true,
        }
    }
}

#[async_trait]
impl StorageUnitOfWork for MockStorageUnitOfWork {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut projects = self.projects_saved.lock().await;
        projects.push(command.project);
        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        _command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        Ok(())
    }

    async fn commit_managed_source_import(
        &self,
        command: ports::transaction::CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut projects = self.projects_saved.lock().await;
        projects.push(command.project);
        Ok(())
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut deleted = self.projects_deleted.lock().await;
        deleted.push(command.project_id);
        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut jobs = self.jobs_saved.lock().await;
        jobs.push(command.job);
        Ok(())
    }

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        command.validate()?;
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut projects = self.projects_saved.lock().await;
        projects.push(command.project);
        let mut jobs = self.jobs_saved.lock().await;
        jobs.push(command.job);
        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        command.validate()?;
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut projects = self.projects_saved.lock().await;
        projects.push(command.project);
        let mut jobs = self.jobs_saved.lock().await;
        jobs.push(command.job);
        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut jobs = self.jobs_saved.lock().await;
        jobs.push(command.job);
        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }
        let mut projects = self.projects_saved.lock().await;
        if let Some(p) = projects.iter_mut().find(|p| p.id() == &command.project_id) {
            let res = p
                .apply_terminal_transition(&command.job_id, command.outcome)
                .map_err(|e| PortError::Unexpected {
                    message: e.to_string(),
                })?;
            Ok(res)
        } else {
            Err(PortError::Unexpected {
                message: "Project not found".to_string(),
            })
        }
    }
}
