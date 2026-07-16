use crate::error::PortError;
use domain::job::Job;
use domain::project::Project;

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
    pub expected_job_revision: u64,
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
