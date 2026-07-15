use async_trait::async_trait;
use std::sync::Arc;

use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitPipelineStart, CommitPipelineStartFailure, CommitProjectDelete,
    CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};

use super::database::InMemoryDatabase;
use std::sync::Mutex;

#[derive(Clone)]
pub struct InMemoryStorageUnitOfWork {
    db: Arc<Mutex<InMemoryDatabase>>,
    artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
    artifact_store: Arc<dyn ports::storage::ArtifactStore>,
}

impl InMemoryStorageUnitOfWork {
    pub fn new(
        db: Arc<Mutex<InMemoryDatabase>>,
        artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
        artifact_store: Arc<dyn ports::storage::ArtifactStore>,
    ) -> Self {
        Self {
            db,
            artifact_index,
            artifact_store,
        }
    }
}

#[async_trait]
impl StorageUnitOfWork for InMemoryStorageUnitOfWork {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        let mut db = self.db.lock().unwrap();
        if !db.projects.contains_key(command.project.id()) {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }
        db.projects
            .insert(command.project.id().clone(), command.project.clone());
        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        // Synchronously finalize artifact for dev mode
        self.artifact_store
            .finalize_staged_artifact(&command.staging_key, &command.final_key)
            .await?;

        let mut artifact = command.artifact;
        artifact.state = domain::media::ArtifactState::Ready;

        self.artifact_index
            .add(&command.project_id, &artifact)
            .await?;

        Ok(())
    }

    async fn commit_managed_source_import(
        &self,
        command: ports::transaction::CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        // Synchronously finalize artifact for dev mode
        self.artifact_store
            .finalize_staged_artifact(&command.staging_key, &command.final_key)
            .await?;

        let mut artifact = command.artifact;
        artifact.state = domain::media::ArtifactState::Ready;

        {
            let mut db = self.db.lock().unwrap();
            if !db.projects.contains_key(command.project.id()) {
                return Err(PortError::NotFound {
                    resource: "Project".to_string(),
                });
            }
            db.projects
                .insert(command.project.id().clone(), command.project.clone());
        }

        self.artifact_index
            .add(command.project.id(), &artifact)
            .await?;

        Ok(())
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        let mut db = self.db.lock().unwrap();
        db.projects.remove(&command.project_id);
        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        let mut db = self.db.lock().unwrap();
        if !db.jobs.contains_key(command.job.id()) {
            return Err(PortError::NotFound {
                resource: "Job".to_string(),
            });
        }
        db.jobs
            .insert(command.job.id().clone(), command.job.clone());
        Ok(())
    }

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        command.validate()?;

        let mut db = self.db.lock().unwrap();
        if !db.projects.contains_key(command.project.id()) {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }
        if db.jobs.contains_key(command.job.id()) {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Job with id {} already exists", command.job.id()),
            });
        }
        db.projects
            .insert(command.project.id().clone(), command.project.clone());
        db.jobs
            .insert(command.job.id().clone(), command.job.clone());
        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut db = self.db.lock().unwrap();
        if !db.projects.contains_key(command.project.id()) {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }
        if !db.jobs.contains_key(command.job.id()) {
            return Err(PortError::NotFound {
                resource: "Job".to_string(),
            });
        }
        db.projects
            .insert(command.project.id().clone(), command.project.clone());
        db.jobs
            .insert(command.job.id().clone(), command.job.clone());
        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        let mut db = self.db.lock().unwrap();
        if !db.jobs.contains_key(command.job.id()) {
            return Err(PortError::NotFound {
                resource: "Job".to_string(),
            });
        }
        db.jobs
            .insert(command.job.id().clone(), command.job.clone());
        // InMemoryAdapter doesn't have an outbox right now, so we just return
        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        let mut db = self.db.lock().unwrap();
        let project =
            db.projects
                .get(&command.project_id)
                .ok_or_else(|| PortError::Unexpected {
                    message: "Project not found".to_string(),
                })?;

        let mut updated_project = project.clone();
        let res = updated_project
            .apply_terminal_transition(&command.job_id, command.outcome)
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

        if matches!(
            res,
            domain::project::status::TerminalTransitionResult::Applied { .. }
        ) {
            db.projects
                .insert(updated_project.id().clone(), updated_project);
        }

        Ok(res)
    }
}
