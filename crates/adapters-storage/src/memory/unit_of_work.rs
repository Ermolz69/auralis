use async_trait::async_trait;
use std::sync::Arc;

use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitPipelineStart, CommitPipelineStartFailure, CommitProjectDelete,
    CommitProjectDeleteResult, CommitStagedArtifactWrite, CommitTranscriptImport,
    StorageUnitOfWork,
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

    fn lock_db(&self) -> Result<std::sync::MutexGuard<'_, InMemoryDatabase>, PortError> {
        self.db.lock().map_err(|_| PortError::Storage {
            operation: "lock_in_memory_uow",
            message: "Mutex poisoned".to_string(),
        })
    }
}

#[async_trait]
impl StorageUnitOfWork for InMemoryStorageUnitOfWork {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        let mut db = self.lock_db()?;
        let existing =
            db.projects
                .get(command.project.id())
                .ok_or_else(|| PortError::NotFound {
                    resource: "Project".to_string(),
                })?;

        // Revalidate the fence
        if existing.updated_at() != command.expected_project_updated_at
            || existing.status() != &command.expected_status
            || existing.active_job_id() != command.expected_active_job_id.as_ref()
        {
            return Err(PortError::Conflict {
                resource: "Project".to_string(),
                message: "Project state changed concurrently".to_string(),
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

    /// NOTE: Memory UoW provides test/compile parity only and does not claim transactional equivalence with SQLite.
    async fn commit_managed_source_import(
        &self,
        command: ports::transaction::CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        command.validate()?;

        let artifact = command.artifact;

        {
            let mut db = self.lock_db()?;
            let existing =
                db.projects
                    .get(command.project.id())
                    .ok_or_else(|| PortError::NotFound {
                        resource: format!("Project {}", command.project.id()),
                    })?;

            if existing.status() != &domain::project::ProjectStatus::Draft {
                return Err(PortError::Conflict {
                    resource: format!("Project {}", command.project.id()),
                    message: "Project is not in Draft status".to_string(),
                });
            }

            if existing.active_job_id().is_some() {
                return Err(PortError::Conflict {
                    resource: format!("Project {}", command.project.id()),
                    message: "Project has active job".to_string(),
                });
            }

            if existing.updated_at() != command.original_updated_at {
                return Err(PortError::Conflict {
                    resource: format!("Project {}", command.project.id()),
                    message: "updated_at mismatch".to_string(),
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

    async fn commit_project_delete(
        &self,
        command: CommitProjectDelete,
    ) -> Result<CommitProjectDeleteResult, PortError> {
        let mut db = self.lock_db()?;

        if !db.projects.contains_key(&command.project_id) {
            return Err(PortError::NotFound {
                resource: format!("Project {}", command.project_id),
            });
        }

        let deleted_job_ids: Vec<domain::job::JobId> = db
            .jobs
            .values()
            .filter(|j| j.project_id() == &command.project_id)
            .map(|j| j.id().clone())
            .collect();

        for job_id in &deleted_job_ids {
            db.jobs.remove(job_id);
        }

        db.projects.remove(&command.project_id);

        Ok(CommitProjectDeleteResult { deleted_job_ids })
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        let mut db = self.lock_db()?;
        if let Some(existing) = db.jobs.get(command.job.id()) {
            if existing.revision() != command.expected_revision {
                return Err(PortError::Conflict {
                    resource: "Job".to_string(),
                    message: format!(
                        "Optimistic concurrency conflict for job id {}",
                        command.job.id()
                    ),
                });
            }
        } else {
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

        let mut db = self.lock_db()?;
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

        let mut db = self.lock_db()?;
        if !db.projects.contains_key(command.project.id()) {
            return Err(PortError::NotFound {
                resource: "Project".to_string(),
            });
        }
        if let Some(existing) = db.jobs.get(command.job.id()) {
            if existing.revision() != command.expected_job_revision {
                return Err(PortError::Conflict {
                    resource: "Job".to_string(),
                    message: format!(
                        "Optimistic concurrency conflict for job id {}",
                        command.job.id()
                    ),
                });
            }
        } else {
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
        let mut db = self.lock_db()?;
        if let Some(existing) = db.jobs.get(command.job.id()) {
            if existing.revision() != command.expected_revision {
                return Err(PortError::Conflict {
                    resource: "Job".to_string(),
                    message: format!(
                        "Optimistic concurrency conflict for job id {}",
                        command.job.id()
                    ),
                });
            }
        } else {
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
        let mut db = self.lock_db()?;
        let project = db
            .projects
            .get(&command.project_id)
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Project {}", command.project_id),
            })?;

        let mut updated_project = project.clone();
        let res = updated_project
            .apply_terminal_transition(&command.job_id, command.outcome)
            .map_err(|e| PortError::Conflict {
                resource: "Project Transition".to_string(),
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

    async fn commit_artifact_finalize(
        &self,
        _command: ports::transaction::CommitArtifactFinalize,
    ) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
        Ok(ports::transaction::CommitArtifactFinalizeResult::Committed)
    }
}
