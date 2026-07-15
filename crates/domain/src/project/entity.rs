use crate::error::DomainError;
use crate::job::{JobId, TerminalOutcome};
use crate::media::{MediaMetadata, MediaSource};
use crate::project::id::ProjectId;
use crate::project::snapshot;
use crate::project::status::{ProjectStatus, TerminalTransitionResult};
use crate::transcript::Transcript;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LanguageCode(pub String);

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    id: ProjectId,
    title: String,
    status: ProjectStatus,
    source: Option<MediaSource>,
    metadata: Option<MediaMetadata>,
    source_language: Option<LanguageCode>,
    target_language: Option<LanguageCode>,
    transcript: Option<Transcript>,
    active_job_id: Option<JobId>,
    last_terminal_job_id: Option<JobId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Project {
    pub fn to_snapshot(&self) -> snapshot::ProjectSnapshot {
        snapshot::ProjectSnapshot {
            id: self.id.clone(),
            title: self.title.clone(),
            status: self.status.clone(),
            source: self.source.clone(),
            metadata: self.metadata.clone(),
            source_language: self.source_language.clone(),
            target_language: self.target_language.clone(),
            transcript: self.transcript.clone(),
            active_job_id: self.active_job_id.clone(),
            last_terminal_job_id: self.last_terminal_job_id.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn from_snapshot(snapshot: snapshot::ProjectSnapshot) -> Result<Self, DomainError> {
        if snapshot.title.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Project title cannot be empty".to_string(),
            ));
        }

        if snapshot
            .source_language
            .as_ref()
            .is_some_and(|c| c.0.trim().is_empty())
        {
            return Err(DomainError::ValidationError(
                "source_language cannot be empty".to_string(),
            ));
        }

        if snapshot
            .target_language
            .as_ref()
            .is_some_and(|c| c.0.trim().is_empty())
        {
            return Err(DomainError::ValidationError(
                "target_language cannot be empty".to_string(),
            ));
        }

        match snapshot.status {
            ProjectStatus::SourceImported
            | ProjectStatus::ReadyForProcessing
            | ProjectStatus::Processing
            | ProjectStatus::Completed
                if snapshot.source.is_none() =>
            {
                return Err(DomainError::ValidationError(format!(
                    "Project in status {:?} must have a source",
                    snapshot.status
                )));
            }
            _ => {}
        }

        Ok(Self {
            id: snapshot.id,
            title: snapshot.title,
            status: snapshot.status,
            source: snapshot.source,
            metadata: snapshot.metadata,
            source_language: snapshot.source_language,
            target_language: snapshot.target_language,
            transcript: snapshot.transcript,
            active_job_id: snapshot.active_job_id,
            last_terminal_job_id: snapshot.last_terminal_job_id,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
        })
    }

    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: ProjectId::new(),
            title,
            status: ProjectStatus::Draft,
            source: None,
            metadata: None,
            source_language: None,
            target_language: None,
            transcript: None,
            active_job_id: None,
            last_terminal_job_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    // Getters
    pub fn id(&self) -> &ProjectId {
        &self.id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn status(&self) -> &ProjectStatus {
        &self.status
    }
    pub fn source(&self) -> Option<&MediaSource> {
        self.source.as_ref()
    }
    pub fn metadata(&self) -> Option<&MediaMetadata> {
        self.metadata.as_ref()
    }
    pub fn source_language(&self) -> Option<&LanguageCode> {
        self.source_language.as_ref()
    }
    pub fn target_language(&self) -> Option<&LanguageCode> {
        self.target_language.as_ref()
    }
    pub fn transcript(&self) -> Option<&Transcript> {
        self.transcript.as_ref()
    }
    pub fn active_job_id(&self) -> Option<&JobId> {
        self.active_job_id.as_ref()
    }
    pub fn last_terminal_job_id(&self) -> Option<&JobId> {
        self.last_terminal_job_id.as_ref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    // Setters for basic info
    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.updated_at = Utc::now();
    }

    pub fn set_languages(&mut self, source: Option<LanguageCode>, target: Option<LanguageCode>) {
        self.source_language = source;
        self.target_language = target;
        self.updated_at = Utc::now();
    }

    pub fn set_transcript(&mut self, transcript: Transcript) {
        self.transcript = Some(transcript);
        self.updated_at = Utc::now();
    }

    // Transitions
    pub fn import_source(
        &mut self,
        source: MediaSource,
        metadata: Option<MediaMetadata>,
    ) -> Result<(), DomainError> {
        if self.status != ProjectStatus::Draft {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "SourceImported".to_string(),
            });
        }
        self.source = Some(source);
        self.metadata = metadata;
        self.status = ProjectStatus::SourceImported;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn mark_ready_for_processing(&mut self) -> Result<(), DomainError> {
        if self.status != ProjectStatus::SourceImported {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "ReadyForProcessing".to_string(),
            });
        }
        self.status = ProjectStatus::ReadyForProcessing;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn start_processing(&mut self, job_id: JobId) -> Result<(), DomainError> {
        match self.status {
            ProjectStatus::ReadyForProcessing | ProjectStatus::Failed => {
                self.status = ProjectStatus::Processing;
                self.active_job_id = Some(job_id);
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Processing".to_string(),
            }),
        }
    }

    pub fn apply_terminal_transition(
        &mut self,
        job_id: &JobId,
        outcome: TerminalOutcome,
    ) -> Result<TerminalTransitionResult, DomainError> {
        let target_status = match outcome {
            TerminalOutcome::Completed => ProjectStatus::Completed,
            TerminalOutcome::Failed => ProjectStatus::Failed,
            TerminalOutcome::Cancelled => ProjectStatus::Cancelled,
        };

        if self.active_job_id.as_ref() != Some(job_id) {
            // Check for idempotency: was this the exact job that last terminalized the project?
            if self.last_terminal_job_id.as_ref() == Some(job_id) {
                if self.status == target_status {
                    return Ok(TerminalTransitionResult::AlreadyApplied);
                } else {
                    return Err(DomainError::InvalidStateTransition {
                        from: format!("{:?}", self.status),
                        to: format!(
                            "Conflicting outcome for already terminal job: {:?}",
                            target_status
                        ),
                    });
                }
            }
            return Ok(TerminalTransitionResult::IgnoredStale);
        }

        if self.status != ProjectStatus::Processing {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", target_status),
            });
        }

        self.status = target_status;
        self.last_terminal_job_id = self.active_job_id.take();
        self.updated_at = Utc::now();

        let transcript_ready = self.transcript.is_some();
        Ok(TerminalTransitionResult::Applied { transcript_ready })
    }

    pub fn force_failed_due_to_recovery(&mut self) {
        self.status = ProjectStatus::Failed;
        self.active_job_id = None;
        self.updated_at = Utc::now();
    }

    #[cfg(test)]
    pub fn set_status(&mut self, status: ProjectStatus) {
        self.status = status;
    }

    #[cfg(test)]
    pub fn set_active_job_id(&mut self, id: JobId) {
        self.active_job_id = Some(id);
    }
}
