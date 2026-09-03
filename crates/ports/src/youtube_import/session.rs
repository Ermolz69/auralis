use domain::{outbox::WorkspaceKey, project::ProjectSnapshot};
use serde::{Deserialize, Serialize};

use crate::transaction::CommitStagedArtifactWrite;

use super::YoutubeImportState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoutubeImportSession {
    pub project: ProjectSnapshot,
    pub original_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub workspace_key: WorkspaceKey,
    pub write: Option<CommitStagedArtifactWrite>,
    pub state: YoutubeImportState,
    pub revision: u64,
}

impl YoutubeImportSession {
    pub fn request_key(&self) -> String {
        match self.original_updated_at {
            Some(_) => format!("project:{}", self.project.id),
            None => format!("url:{}", self.url()),
        }
    }

    pub fn url(&self) -> &str {
        match &self.project.source {
            Some(domain::media::MediaSource::YoutubeUrl { url }) => url,
            _ => "",
        }
    }
}
