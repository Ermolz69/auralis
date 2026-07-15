use chrono::{DateTime, Utc};

use crate::dubbing::DubbingPipelineStage;
use crate::error::DomainError;
use crate::project::ProjectId;

use super::{JobError, JobId, JobKind, JobProgress, JobSnapshot, JobStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    id: JobId,
    project_id: ProjectId,
    title: String,
    kind: JobKind,
    status: JobStatus,
    stage: Option<DubbingPipelineStage>,
    progress: JobProgress,
    error: Option<JobError>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

impl Job {
    pub fn new(project_id: ProjectId, title: String, kind: JobKind) -> Self {
        let now = Utc::now();
        Self {
            id: JobId::new(),
            project_id,
            title,
            kind,
            status: JobStatus::Pending,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn id(&self) -> &JobId {
        &self.id
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn kind(&self) -> &JobKind {
        &self.kind
    }

    pub fn status(&self) -> &JobStatus {
        &self.status
    }

    pub fn stage(&self) -> Option<&DubbingPipelineStage> {
        self.stage.as_ref()
    }

    pub fn progress(&self) -> &JobProgress {
        &self.progress
    }

    pub fn error(&self) -> Option<&JobError> {
        self.error.as_ref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub fn started_at(&self) -> Option<&DateTime<Utc>> {
        self.started_at.as_ref()
    }

    pub fn finished_at(&self) -> Option<&DateTime<Utc>> {
        self.finished_at.as_ref()
    }

    pub fn start(&mut self) -> Result<(), DomainError> {
        if self.status != JobStatus::Pending {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Running".to_string(),
            });
        }

        let now = Utc::now();
        self.status = JobStatus::Running;
        self.started_at = Some(now);
        self.updated_at = now;
        self.progress.message = "Job started".to_string();

        Ok(())
    }

    pub fn advance(
        &mut self,
        stage: DubbingPipelineStage,
        progress: JobProgress,
    ) -> Result<(), DomainError> {
        if self.status != JobStatus::Running {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Stage Update (Running)".to_string(),
            });
        }

        progress.validate()?;

        self.stage = Some(stage);
        self.progress = progress;
        self.updated_at = Utc::now();

        Ok(())
    }

    pub fn mark_completed(&mut self) -> Result<(), DomainError> {
        if self.status != JobStatus::Running {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Completed".to_string(),
            });
        }

        let now = Utc::now();
        self.status = JobStatus::Completed;
        self.progress.percent = 100;
        self.progress.message = "Job completed successfully".to_string();
        self.finished_at = Some(now);
        self.updated_at = now;

        Ok(())
    }

    pub fn mark_failed(&mut self, error: JobError) -> Result<(), DomainError> {
        if matches!(self.status, JobStatus::Completed | JobStatus::Cancelled) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Failed".to_string(),
            });
        }

        let now = Utc::now();
        self.status = JobStatus::Failed;
        self.error = Some(error);
        self.finished_at = Some(now);
        self.updated_at = now;

        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), DomainError> {
        if self.status == JobStatus::Cancelled {
            return Ok(());
        }

        if matches!(self.status, JobStatus::Completed | JobStatus::Failed) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Cancelled".to_string(),
            });
        }

        let now = Utc::now();
        self.status = JobStatus::Cancelled;
        self.finished_at = Some(now);
        self.updated_at = now;

        Ok(())
    }

    pub fn to_snapshot(&self) -> JobSnapshot {
        JobSnapshot {
            id: self.id.clone(),
            project_id: self.project_id.clone(),
            title: self.title.clone(),
            kind: self.kind.clone(),
            status: self.status.clone(),
            stage: self.stage.clone(),
            progress: self.progress.clone(),
            error: self.error.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
        }
    }

    pub fn from_snapshot(snapshot: JobSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            title: snapshot.title,
            kind: snapshot.kind,
            status: snapshot.status,
            stage: snapshot.stage,
            progress: snapshot.progress,
            error: snapshot.error,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            started_at: snapshot.started_at,
            finished_at: snapshot.finished_at,
        }
    }

    #[cfg(test)]
    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status;
    }

    #[cfg(test)]
    pub fn set_id(&mut self, id: JobId) {
        self.id = id;
    }
}
