use async_trait::async_trait;

use crate::error::PortError;
use domain::job::Job;
use domain::media::Artifact;
use domain::project::{Project, ProjectId};

pub struct CommitTranscriptImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_workspace_key: Option<String>,
}

pub struct CommitManagedSourceImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
}

impl CommitManagedSourceImport {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.project.status() != &domain::project::ProjectStatus::SourceImported {
            return Err(PortError::Unexpected {
                message: "Project must be in SourceImported status".to_string(),
            });
        }
        if self.artifact.kind != domain::media::ArtifactKind::SourceVideo {
            return Err(PortError::Unexpected {
                message: "Artifact must be SourceVideo".to_string(),
            });
        }
        if self.artifact.state != domain::media::ArtifactState::PendingFinalize {
            return Err(PortError::Unexpected {
                message: "Artifact must be PendingFinalize".to_string(),
            });
        }
        match self.project.source() {
            Some(domain::media::MediaSource::ManagedLocalFile { artifact_id, .. }) => {
                if artifact_id != &self.artifact.id {
                    return Err(PortError::Unexpected {
                        message: "Project source artifact ID does not match artifact".to_string(),
                    });
                }
            }
            _ => {
                return Err(PortError::Unexpected {
                    message: "Project source must be ManagedLocalFile".to_string(),
                });
            }
        }
        Ok(())
    }
}

pub struct CommitStagedArtifactWrite {
    pub project_id: ProjectId,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_workspace_key: Option<String>,
}

pub struct CommitProjectDelete {
    pub project_id: ProjectId,
    pub artifacts: Vec<Artifact>,
}

pub struct CommitJobUpdate {
    pub job: Job,
}

pub struct CommitPipelineStart {
    pub project: Project,
    pub job: Job,
}

impl CommitPipelineStart {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.job.project_id() != self.project.id() {
            return Err(PortError::Unexpected {
                message: "Job does not belong to the project".to_string(),
            });
        }
        if self.project.status() != &domain::project::ProjectStatus::Processing {
            return Err(PortError::Unexpected {
                message: "Project must be in Processing status".to_string(),
            });
        }
        if self.job.status() != &domain::job::JobStatus::Pending {
            return Err(PortError::Unexpected {
                message: "Job must be in Pending status".to_string(),
            });
        }
        Ok(())
    }
}

pub struct CommitPipelineStartFailure {
    pub project: Project,
    pub job: Job,
}

impl CommitPipelineStartFailure {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.job.project_id() != self.project.id() {
            return Err(PortError::Unexpected {
                message: "Job does not belong to the project".to_string(),
            });
        }
        if self.project.status() != &domain::project::ProjectStatus::Failed {
            return Err(PortError::Unexpected {
                message: "Project must be in Failed status".to_string(),
            });
        }
        if self.job.status() != &domain::job::JobStatus::Failed {
            return Err(PortError::Unexpected {
                message: "Job must be in Failed status".to_string(),
            });
        }
        Ok(())
    }
}

pub struct CommitTerminalJobUpdate {
    pub job: Job,
    pub outbox_message_id: String,
    pub project_id: ProjectId,
    pub outcome: domain::job::TerminalOutcome,
}

pub struct ApplyTerminalLifecycle {
    pub project_id: ProjectId,
    pub job_id: domain::job::JobId,
    pub outcome: domain::job::TerminalOutcome,
}

#[async_trait]
pub trait StorageUnitOfWork: Send + Sync {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError>;

    async fn commit_staged_artifact_write(
        &self,
        data: CommitStagedArtifactWrite,
    ) -> Result<(), PortError>;

    async fn commit_managed_source_import(
        &self,
        data: CommitManagedSourceImport,
    ) -> Result<(), PortError>;

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError>;

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError>;

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError>;

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError>;

    async fn commit_terminal_job_update(
        &self,
        command: CommitTerminalJobUpdate,
    ) -> Result<(), PortError>;

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError>;
}

#[async_trait]
impl<T: ?Sized + StorageUnitOfWork> StorageUnitOfWork for std::sync::Arc<T> {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        (**self).commit_transcript_import(command).await
    }

    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        (**self).commit_staged_artifact_write(command).await
    }

    async fn commit_managed_source_import(
        &self,
        command: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        (**self).commit_managed_source_import(command).await
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        (**self).commit_project_delete(command).await
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        (**self).commit_job_update(command).await
    }

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        (**self).commit_pipeline_start(command).await
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        (**self).commit_pipeline_start_failure(command).await
    }

    async fn commit_terminal_job_update(
        &self,
        command: CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        (**self).commit_terminal_job_update(command).await
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        (**self)
            .apply_terminal_lifecycle_conditionally(command)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::{Job, JobId, JobKind};
    use domain::project::Project;

    #[test]
    fn test_commit_pipeline_start_validate_success() {
        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "path".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let job_id = JobId::new();
        project.start_processing(job_id.clone()).unwrap();

        let job = Job::new(
            project.id().clone(),
            "Test Job".to_string(),
            JobKind::Dubbing,
        );

        let command = CommitPipelineStart { project, job };
        assert!(command.validate().is_ok());
    }

    #[test]
    fn test_commit_pipeline_start_validate_mismatched_project_id() {
        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "path".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let job_id = JobId::new();
        project.start_processing(job_id.clone()).unwrap();

        // Job has a completely different project ID
        let wrong_project_id = domain::project::ProjectId::new();
        let job = Job::new(wrong_project_id, "Test Job".to_string(), JobKind::Dubbing);

        let command = CommitPipelineStart { project, job };
        let err = command.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Job does not belong to the project")
        );
    }

    #[test]
    fn test_commit_pipeline_start_validate_wrong_project_status() {
        let project = Project::new("Test".to_string());
        let job = Job::new(
            project.id().clone(),
            "Test Job".to_string(),
            JobKind::Dubbing,
        );

        let command = CommitPipelineStart { project, job };
        let err = command.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Project must be in Processing status")
        );
    }

    #[test]
    fn test_commit_pipeline_start_validate_wrong_job_status() {
        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "path".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let job_id = JobId::new();
        project.start_processing(job_id.clone()).unwrap();

        let mut job = Job::new(
            project.id().clone(),
            "Test Job".to_string(),
            JobKind::Dubbing,
        );
        job.start().unwrap();

        let command = CommitPipelineStart { project, job };
        let err = command.validate().unwrap_err();
        assert!(err.to_string().contains("Job must be in Pending status"));
    }

    #[test]
    fn test_commit_pipeline_start_failure_validate_mismatched_project_id() {
        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "path".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let job_id = JobId::new();
        project.start_processing(job_id.clone()).unwrap();
        project
            .apply_terminal_transition(&job_id, domain::job::TerminalOutcome::Failed)
            .unwrap();

        let wrong_project_id = domain::project::ProjectId::new();
        let mut job = Job::new(wrong_project_id, "Test Job".to_string(), JobKind::Dubbing);
        job.mark_failed(domain::job::JobError::new("TEST", "error", false))
            .unwrap();

        let command = CommitPipelineStartFailure { project, job };
        let err = command.validate().unwrap_err();
        assert!(
            err.to_string()
                .contains("Job does not belong to the project")
        );
    }
}
