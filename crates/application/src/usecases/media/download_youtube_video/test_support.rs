#![allow(clippy::unwrap_used, clippy::expect_used)]
use async_trait::async_trait;
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, MediaSource};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::source::{DownloadMediaRequest, VideoSourcePort};
use ports::storage::ArtifactStore;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(super) struct MockArtifactStore {
    pub(super) fail_on_stage: Arc<AtomicBool>,
    pub(super) deleted_keys: Arc<std::sync::Mutex<Vec<String>>>,
}

#[async_trait]
impl ArtifactStore for MockArtifactStore {
    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, PortError> {
        if self.fail_on_stage.load(Ordering::SeqCst) {
            return Err(PortError::Io {
                message: "Simulated stage error".into(),
            });
        }

        let ext = filename_hint
            .and_then(|h| std::path::Path::new(h).extension())
            .or_else(|| source_path.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let artifact = Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::DownloadedVideo,
            location: ArtifactLocation::StorageKey(format!("final.{}", ext)),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: None,
        };

        Ok(ports::storage::StagedArtifact {
            artifact,
            staging_key: format!("staging.{}", ext),
            final_key: format!("final.{}", ext),
            size_bytes: 1024,
        })
    }

    async fn import_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, PortError> {
        unimplemented!()
    }

    async fn cleanup_stale_staging(&self, _max_age: std::time::Duration) -> Result<(), PortError> {
        unimplemented!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_artifact(&self, _artifact: &Artifact) -> Result<PathBuf, PortError> {
        Ok(PathBuf::from("/tmp/artifact"))
    }

    async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError> {
        self.deleted_keys
            .lock()
            .unwrap()
            .push(storage_key.to_string());
        Ok(())
    }

    async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct FailingVideoSourceAdapter;

#[async_trait]
impl VideoSourcePort for FailingVideoSourceAdapter {
    async fn validate_source(&self, _source: &MediaSource) -> Result<(), PortError> {
        Ok(())
    }
    async fn fetch_metadata(
        &self,
        _source: &MediaSource,
    ) -> Result<domain::media::MediaMetadata, PortError> {
        unimplemented!()
    }
    async fn download_media(&self, _request: DownloadMediaRequest) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: ArtifactId::new(),
            kind: ArtifactKind::DownloadedVideo,
            location: ArtifactLocation::StorageKey("this_should_be_local_path".to_string()),
            size_bytes: None,
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: None,
        })
    }
}
