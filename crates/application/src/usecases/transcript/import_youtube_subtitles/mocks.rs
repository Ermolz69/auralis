#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use domain::media::{Artifact, ArtifactKind, MediaSource, SubtitleTrack};
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};
use ports::storage::{ArtifactStore, StagedArtifact};
use ports::workspace::{TempWorkspacePort, WorkspaceAllocation, WorkspaceCleanupReport};

pub(crate) struct MockProjectRepo {
    pub(crate) project: Option<Project>,
}
#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        Ok(project)
    }
    async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
        Ok(self.project.clone())
    }
    async fn save(&self, _project: &Project) -> Result<(), PortError> {
        Ok(())
    }
    async fn list(&self) -> Result<Vec<Project>, PortError> {
        Ok(vec![])
    }
    async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
}

pub(crate) struct MockSubtitleSource {
    pub(crate) fail_download: bool,
}
#[async_trait]
impl SubtitleSourcePort for MockSubtitleSource {
    async fn list_subtitles(&self, _source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(vec![SubtitleTrack {
            id: "1".to_string(),
            language: "en".to_string(),
            label: Some("English".to_string()),
            format: Some("vtt".to_string()),
            is_auto_generated: false,
        }])
    }
    async fn download_subtitle(
        &self,
        request: DownloadSubtitleRequest,
    ) -> Result<Artifact, PortError> {
        if self.fail_download {
            return Err(PortError::Io {
                message: "download failed".into(),
            });
        }
        let file_path = request.target_directory.join("subs.vtt");
        std::fs::write(&file_path, "WEBVTT\n\n00:00.000 --> 00:01.000\nHello").unwrap();
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: domain::media::ArtifactLocation::LocalPath(
                file_path.to_string_lossy().to_string(),
            ),
            size_bytes: Some(10),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: None,
        })
    }
}

pub(crate) struct MockArtifactStoreForSubs {
    pub(crate) fail_delete: bool,
    pub(crate) deleted_keys: Arc<Mutex<Vec<String>>>,
}
#[async_trait]
impl ArtifactStore for MockArtifactStoreForSubs {
    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        kind: ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        Ok(StagedArtifact {
            artifact: Artifact {
                id: domain::media::ArtifactId::new(),
                kind,
                location: domain::media::ArtifactLocation::StorageKey("final".into()),
                size_bytes: Some(10),
                state: domain::media::ArtifactState::PendingFinalize,
                created_at: domain::chrono::Utc::now(),
                updated_at: domain::chrono::Utc::now(),
                ready_at: None,
            },
            staging_key: "staging_key".into(),
            final_key: "final_key".into(),
            size_bytes: 10,
        })
    }
    async fn import_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unimplemented!()
    }

    async fn stage_owned_workspace_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        _workspace_port: &dyn TempWorkspacePort,
        _allocation_key: &domain::outbox::WorkspaceKey,
        _relative_file: &str,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        let dummy_path = std::path::Path::new("dummy");
        self.stage_owned_temp_file(project_id, kind, dummy_path, filename_hint)
            .await
    }
    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn resolve_artifact(
        &self,
        _artifact: &Artifact,
    ) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
    }
    async fn delete_storage_key(&self, key: &str) -> Result<(), PortError> {
        if self.fail_delete {
            return Err(PortError::Io {
                message: "delete failed".into(),
            });
        }
        self.deleted_keys.lock().unwrap().push(key.to_string());
        Ok(())
    }
    async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
        Ok(())
    }
    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
    async fn cleanup_stale_staging(&self, _max_age: std::time::Duration) -> Result<(), PortError> {
        Ok(())
    }
}

pub(crate) struct MockWorkspacePortForSubs {
    pub(crate) fail_delete: bool,
    pub(crate) deleted_keys: Arc<Mutex<Vec<String>>>,
    pub(crate) allocated_path: Arc<Mutex<Option<std::path::PathBuf>>>,
}
#[async_trait]
impl TempWorkspacePort for MockWorkspacePortForSubs {
    async fn create_allocation(
        &self,
        _project_id: &ProjectId,
        _purpose: &str,
    ) -> Result<WorkspaceAllocation, PortError> {
        let path = std::env::temp_dir().join(format!(
            "test_alloc_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        *self.allocated_path.lock().unwrap() = Some(path.clone());
        Ok(WorkspaceAllocation {
            absolute_path: path,
            workspace_key: domain::outbox::WorkspaceKey::new("tmp/1/subs").unwrap(),
            allocation_id: "1".into(),
        })
    }
    async fn delete_allocation(&self, key: &domain::outbox::WorkspaceKey) -> Result<(), PortError> {
        if self.fail_delete {
            return Err(PortError::Io {
                message: "workspace delete failed".into(),
            });
        }
        self.deleted_keys
            .lock()
            .unwrap()
            .push(key.as_str().to_string());
        if let Some(path) = self.allocated_path.lock().unwrap().take() {
            let _ = std::fs::remove_dir_all(path);
        }
        Ok(())
    }
    async fn resolve_key(
        &self,
        _key: &domain::outbox::WorkspaceKey,
    ) -> Result<std::path::PathBuf, PortError> {
        let p = self
            .allocated_path
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default(); // allow-fallback
        Ok(p)
    }
    async fn cleanup_stale_allocations(
        &self,
        _age_threshold: std::time::Duration,
    ) -> Result<WorkspaceCleanupReport, PortError> {
        Ok(WorkspaceCleanupReport {
            deleted_count: 0,
            failed_count: 0,
        })
    }
    async fn read_workspace_file_to_string(
        &self,
        _allocation_key: &domain::outbox::WorkspaceKey,
        relative_file: &str,
        _max_bytes: u64,
    ) -> Result<String, PortError> {
        let p = self
            .allocated_path
            .lock()
            .unwrap()
            .clone()
            .ok_or(PortError::Io {
                message: "No allocated path".into(),
            })?;
        let file_path = p.join(relative_file);
        std::fs::read_to_string(file_path).map_err(|e| PortError::Io {
            message: e.to_string(),
        })
    }
    async fn resolve_child_path(
        &self,
        _key: &domain::outbox::WorkspaceKey,
        relative_file: &str,
    ) -> Result<std::path::PathBuf, PortError> {
        let p = self
            .allocated_path
            .lock()
            .unwrap()
            .clone()
            .ok_or(PortError::Io {
                message: "No allocated path".into(),
            })?;
        Ok(p.join(relative_file))
    }
}
