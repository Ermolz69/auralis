use crate::error::PortError;
use crate::transaction::CommitStagedArtifactWrite;
use domain::media::{ArtifactKind, ArtifactState, MediaSource};
use domain::project::{Project, ProjectStatus};

pub struct CommitYoutubeImport {
    pub project: Project,
    pub write: CommitStagedArtifactWrite,
    pub original_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl CommitYoutubeImport {
    pub fn validate(&self) -> Result<(), PortError> {
        self.write.validate()?;
        let valid_workspace = self.write.temp_workspace_key.as_ref().is_some_and(|key| {
            key.as_str()
                .starts_with(&format!("tmp/{}/", self.project.id()))
                && key.as_str().split('/').count() == 3
        });
        if self.write.project_id != *self.project.id()
            || self.project.status() != &ProjectStatus::ReadyForProcessing
            || !matches!(self.project.source(), Some(MediaSource::YoutubeUrl { .. }))
            || self.project.metadata().is_none()
            || self.project.active_job_id().is_some()
            || self.project.last_terminal_job_id().is_some()
            || self.project.transcript().is_some()
            || self.write.artifact.kind != ArtifactKind::DownloadedVideo
            || self.write.artifact.state != ArtifactState::PendingFinalize
            || !valid_workspace
            || (self.original_updated_at.is_none() && self.project.revision() != 1)
        {
            return Err(PortError::Unexpected {
                message: "Invalid atomic YouTube source import".into(),
            });
        }
        Ok(())
    }
}
