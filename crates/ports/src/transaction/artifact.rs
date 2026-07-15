use crate::error::PortError;
use domain::media::Artifact;
use domain::project::{Project, ProjectId};

pub struct CommitTranscriptImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_workspace_key: Option<domain::outbox::WorkspaceKey>,
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
    pub temp_workspace_key: Option<domain::outbox::WorkspaceKey>,
}
