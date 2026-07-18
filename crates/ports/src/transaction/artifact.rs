use crate::error::PortError;
use domain::media::Artifact;
use domain::project::{Project, ProjectId};

pub struct CommitTranscriptImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub temp_workspace_key: Option<domain::outbox::WorkspaceKey>,
    pub expected_project_updated_at: chrono::DateTime<chrono::Utc>,
    pub expected_status: domain::project::ProjectStatus,
    pub expected_active_job_id: Option<domain::job::JobId>,
}

pub struct CommitManagedSourceImport {
    pub project: Project,
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub original_updated_at: chrono::DateTime<chrono::Utc>,
}

fn is_clean_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    if key.starts_with('/') || key.starts_with('\\') {
        return false;
    }
    if key.contains('\\') || key.contains(':') {
        return false;
    }

    let components: Vec<&str> = key.split('/').collect();
    if components.len() < 2 {
        return false;
    }
    for comp in components {
        if comp.is_empty() || comp == "." || comp == ".." {
            return false;
        }
    }
    true
}

impl CommitManagedSourceImport {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.project.status() != &domain::project::ProjectStatus::ReadyForProcessing {
            return Err(PortError::Unexpected {
                message: "Project must be in ReadyForProcessing status".to_string(),
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
        if self.artifact.location
            != domain::media::ArtifactLocation::StorageKey(self.final_key.clone())
        {
            return Err(PortError::Unexpected {
                message: "Artifact location must match final_key StorageKey".to_string(),
            });
        }
        if !is_clean_key(&self.final_key) {
            return Err(PortError::Unexpected {
                message: "final_key must be a clean relative storage key".to_string(),
            });
        }
        if !is_clean_key(&self.staging_key) {
            return Err(PortError::Unexpected {
                message: "staging_key must be a clean relative storage key".to_string(),
            });
        }

        let project_id_str = self.project.id().to_string();
        let final_first_comp = self.final_key.split('/').next().unwrap_or("");
        if final_first_comp != project_id_str {
            return Err(PortError::Unexpected {
                message: "final_key must start with the project ID".to_string(),
            });
        }

        let staging_first_comp = self.staging_key.split('/').next().unwrap_or("");
        if staging_first_comp != ".staging" {
            return Err(PortError::Unexpected {
                message: "staging_key must start with .staging".to_string(),
            });
        }

        if self.staging_key == self.final_key {
            return Err(PortError::Unexpected {
                message: "staging_key and final_key must be different".to_string(),
            });
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

pub struct CommitArtifactFinalize {
    pub message_id: domain::outbox::OutboxMessageId,
    pub project_id: ProjectId,
    pub artifact_id: domain::media::ArtifactId,
    pub ready_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitArtifactFinalizeResult {
    Committed,
    ObsoleteBecauseProjectDeleted,
    Conflict,
    AlreadyFinalized,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use domain::media::{
        Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState, MediaSource,
    };
    use domain::project::{Project, ProjectId};

    fn create_valid_command() -> (CommitManagedSourceImport, ProjectId, ArtifactId) {
        let project_id = ProjectId::new();
        let artifact_id = ArtifactId::new();
        let final_key = format!("{}/source-video/{}.mp4", project_id, artifact_id);
        let staging_key = format!(".staging/{}/{}.mp4", ArtifactId::new(), artifact_id);

        let mut project = Project::new("Test".into());
        // Force project ID
        let mut snapshot = project.to_snapshot();
        snapshot.id = project_id.clone();
        project = Project::from_snapshot(snapshot).unwrap();

        project
            .import_source(
                MediaSource::ManagedLocalFile {
                    artifact_id: artifact_id.clone(),
                    original_filename: "test.mp4".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();

        let artifact = Artifact {
            id: artifact_id.clone(),
            kind: ArtifactKind::SourceVideo,
            location: ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(100),
            state: ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };

        let cmd = CommitManagedSourceImport {
            project,
            artifact,
            staging_key,
            final_key,
            original_updated_at: chrono::Utc::now(),
        };

        (cmd, project_id, artifact_id)
    }

    #[test]
    fn test_valid_command_accepted() {
        let (cmd, _, _) = create_valid_command();
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_rejects_non_ready_for_processing_status() {
        let (mut cmd, _, _) = create_valid_command();
        let mut snapshot = cmd.project.to_snapshot();
        snapshot.status = domain::project::ProjectStatus::Draft;
        snapshot.source = None;
        cmd.project = Project::from_snapshot(snapshot).unwrap();
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_wrong_artifact_kind() {
        let (mut cmd, _, _) = create_valid_command();
        cmd.artifact.kind = ArtifactKind::ExtractedAudio;
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_non_pending_finalize_state() {
        let (mut cmd, _, _) = create_valid_command();
        cmd.artifact.state = ArtifactState::Ready;
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_artifact_id_mismatch() {
        let (mut cmd, _, _) = create_valid_command();
        cmd.artifact.id = ArtifactId::new();
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_location_final_key_mismatch() {
        let (mut cmd, _, _) = create_valid_command();
        cmd.artifact.location = ArtifactLocation::StorageKey("other/key.mp4".to_string());
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_foreign_project_prefix() {
        let (mut cmd, _, _) = create_valid_command();
        let other_project_id = ProjectId::new();
        cmd.final_key = format!("{}/source-video/{}.mp4", other_project_id, cmd.artifact.id);
        cmd.artifact.location = ArtifactLocation::StorageKey(cmd.final_key.clone());
        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_rejects_path_traversals_and_invalid_keys() {
        let (mut cmd, project_id, artifact_id) = create_valid_command();

        // 1. Single component key (insufficient components)
        cmd.final_key = "project-id".to_string();
        cmd.artifact.location = ArtifactLocation::StorageKey(cmd.final_key.clone());
        assert!(cmd.validate().is_err());

        // 2. .staging single component
        let (mut cmd2, _, _) = create_valid_command();
        cmd2.staging_key = ".staging".to_string();
        assert!(cmd2.validate().is_err());

        // 3. Double slash (empty component)
        let (mut cmd3, _, _) = create_valid_command();
        cmd3.final_key = format!("{}/source-video//{}", project_id, artifact_id);
        cmd3.artifact.location = ArtifactLocation::StorageKey(cmd3.final_key.clone());
        assert!(cmd3.validate().is_err());

        // 4. Dot component (.)
        let (mut cmd4, _, _) = create_valid_command();
        cmd4.final_key = format!("{}/./{}", project_id, artifact_id);
        cmd4.artifact.location = ArtifactLocation::StorageKey(cmd4.final_key.clone());
        assert!(cmd4.validate().is_err());

        // 5. Parent component (..)
        let (mut cmd5, _, _) = create_valid_command();
        cmd5.final_key = format!("{}/../other/{}", project_id, artifact_id);
        cmd5.artifact.location = ArtifactLocation::StorageKey(cmd5.final_key.clone());
        assert!(cmd5.validate().is_err());

        // 6. Absolute Unix path
        let (mut cmd6, _, _) = create_valid_command();
        cmd6.final_key = format!("/{}/source-video/file.mp4", project_id);
        cmd6.artifact.location = ArtifactLocation::StorageKey(cmd6.final_key.clone());
        assert!(cmd6.validate().is_err());

        // 7. Windows drive letter / backslash
        let (mut cmd7, _, _) = create_valid_command();
        cmd7.final_key = format!("C:\\{}\\file.mp4", project_id);
        cmd7.artifact.location = ArtifactLocation::StorageKey(cmd7.final_key.clone());
        assert!(cmd7.validate().is_err());

        // 8. URL-like scheme
        let (mut cmd8, _, _) = create_valid_command();
        cmd8.final_key = format!("s3://bucket/{}/file.mp4", project_id);
        cmd8.artifact.location = ArtifactLocation::StorageKey(cmd8.final_key.clone());
        assert!(cmd8.validate().is_err());
    }

    #[test]
    fn test_allows_hidden_normal_components() {
        let (mut cmd, project_id, artifact_id) = create_valid_command();
        cmd.final_key = format!("{}/.hidden/{}", project_id, artifact_id);
        cmd.artifact.location = ArtifactLocation::StorageKey(cmd.final_key.clone());
        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_rejects_matching_staging_and_final_keys() {
        let (mut cmd, project_id, _artifact_id) = create_valid_command();
        let same_key = format!("{}/file.mp4", project_id);
        cmd.final_key = same_key.clone();
        cmd.staging_key = same_key.clone();
        cmd.artifact.location = ArtifactLocation::StorageKey(same_key);
        assert!(cmd.validate().is_err());
    }
}
