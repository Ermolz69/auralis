mod id;
pub mod snapshot;

pub use id::ProjectId;
pub use snapshot::ProjectSnapshot;

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::media::{Artifact, MediaMetadata, MediaSource};
use crate::transcript::Transcript;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct LanguageCode(pub String);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectStatus {
    Draft,
    SourceImported,
    ReadyForProcessing,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

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
    artifacts: Vec<Artifact>,
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
            artifacts: self.artifacts.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn from_snapshot(snapshot: snapshot::ProjectSnapshot) -> Result<Self, DomainError> {
        Ok(Self {
            id: snapshot.id,
            title: snapshot.title,
            status: snapshot.status,
            source: snapshot.source,
            metadata: snapshot.metadata,
            source_language: snapshot.source_language,
            target_language: snapshot.target_language,
            transcript: snapshot.transcript,
            artifacts: snapshot.artifacts,
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
            artifacts: Vec::new(),
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
    pub fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
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

    pub fn mark_processing_started(&mut self) -> Result<(), DomainError> {
        match self.status {
            ProjectStatus::ReadyForProcessing | ProjectStatus::Failed => {
                self.status = ProjectStatus::Processing;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Processing".to_string(),
            }),
        }
    }

    pub fn mark_completed(&mut self) -> Result<(), DomainError> {
        if self.status != ProjectStatus::Processing {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Completed".to_string(),
            });
        }
        self.status = ProjectStatus::Completed;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn mark_failed(&mut self) -> Result<(), DomainError> {
        if self.status != ProjectStatus::Processing {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Failed".to_string(),
            });
        }
        self.status = ProjectStatus::Failed;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Completed || self.status == ProjectStatus::Cancelled {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Cancelled".to_string(),
            });
        }
        self.status = ProjectStatus::Cancelled;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn add_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_transitions() {
        let mut project = Project::new("Test Video".to_string());
        assert_eq!(project.status(), &ProjectStatus::Draft);

        // Cannot start processing from Draft
        assert!(project.mark_processing_started().is_err());

        // Import source
        let source = MediaSource::RemoteUrl {
            url: "https://example.com/video.mp4".to_string(),
        };
        assert!(project.import_source(source, None).is_ok());
        assert_eq!(project.status(), &ProjectStatus::SourceImported);

        // Mark ready
        assert!(project.mark_ready_for_processing().is_ok());
        assert_eq!(project.status(), &ProjectStatus::ReadyForProcessing);

        // Start processing
        assert!(project.mark_processing_started().is_ok());
        assert_eq!(project.status(), &ProjectStatus::Processing);

        // Complete
        assert!(project.mark_completed().is_ok());
        assert_eq!(project.status(), &ProjectStatus::Completed);

        // Cannot cancel completed project
        assert!(project.cancel().is_err());
    }

    #[test]
    fn test_project_fail_and_retry() {
        let mut project = Project::new("Retry Test".to_string());
        let source = MediaSource::RemoteUrl {
            url: "https://example.com/video.mp4".to_string(),
        };
        project.import_source(source, None).unwrap();
        project.mark_ready_for_processing().unwrap();
        project.mark_processing_started().unwrap();

        // Fail
        assert!(project.mark_failed().is_ok());
        assert_eq!(project.status(), &ProjectStatus::Failed);

        // Retry (start processing again from failed state)
        assert!(project.mark_processing_started().is_ok());
        assert_eq!(project.status(), &ProjectStatus::Processing);
    }
}
