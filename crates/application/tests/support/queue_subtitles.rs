#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use domain::{
    media::{Artifact, ArtifactKind, ArtifactLocation, MediaSource, SubtitleTrack},
    outbox::WorkspaceKey,
    project::ProjectId,
};
use ports::{
    PortError,
    cancellation::CancellationToken,
    source::{DownloadSubtitleRequest, SubtitleSourcePort},
    workspace::{TempWorkspacePort, WorkspaceAllocation, WorkspaceCleanupReport},
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

pub struct Download {
    pub track: SubtitleTrack,
    pub release: oneshot::Sender<bool>,
}

pub async fn downloaded(receive: &mut mpsc::UnboundedReceiver<Download>) -> Download {
    tokio::time::timeout(Duration::from_secs(5), receive.recv())
        .await
        .expect("Pipeline did not reach the subtitle adapter")
        .unwrap()
}

#[derive(Clone)]
pub struct FixtureSubtitles(pub mpsc::UnboundedSender<Download>);

#[async_trait::async_trait]
impl SubtitleSourcePort for FixtureSubtitles {
    async fn list_subtitles(&self, _: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(vec![track()])
    }
    async fn download_subtitle(
        &self,
        request: DownloadSubtitleRequest,
    ) -> Result<Artifact, PortError> {
        self.download_subtitle_cancellable(request, &CancellationToken::new())
            .await
    }
    async fn download_subtitle_cancellable(
        &self,
        request: DownloadSubtitleRequest,
        token: &CancellationToken,
    ) -> Result<Artifact, PortError> {
        let (release, receive) = oneshot::channel();
        self.0
            .send(Download {
                track: request.track,
                release,
            })
            .unwrap();
        let success = tokio::select! {
            result = receive => result.unwrap_or(false),
            _ = token.cancelled() => return Err(PortError::Cancelled),
        };
        if !success {
            return Err(PortError::Io {
                message: "Local fixture failure".into(),
            });
        }
        let path = request.target_directory.join("subtitle.vtt");
        tokio::fs::write(&path, "WEBVTT\n\n00:00.000 --> 00:01.000\nHello fixture\n")
            .await
            .unwrap();
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::LocalPath(path.to_string_lossy().into()),
            size_bytes: None,
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        })
    }
}

pub fn track() -> SubtitleTrack {
    SubtitleTrack {
        id: "selected-auto-ru".into(),
        language: "ru".into(),
        label: Some("Frozen track".into()),
        format: Some("vtt".into()),
        is_auto_generated: true,
    }
}

pub struct CleanupWorkspace {
    pub inner: Arc<adapters_storage::local::LocalTempWorkspace>,
    pub entered: CancellationToken,
    pub release: CancellationToken,
    pub fail: bool,
}

#[async_trait::async_trait]
impl TempWorkspacePort for CleanupWorkspace {
    async fn create_allocation(
        &self,
        id: &ProjectId,
        purpose: &str,
    ) -> Result<WorkspaceAllocation, PortError> {
        self.inner.create_allocation(id, purpose).await
    }
    async fn delete_allocation(&self, key: &WorkspaceKey) -> Result<(), PortError> {
        self.entered.cancel();
        self.release.cancelled().await;
        if self.fail {
            return Err(PortError::Io {
                message: "Fixture cleanup failure".into(),
            });
        }
        self.inner.delete_allocation(key).await
    }
    async fn resolve_key(&self, key: &WorkspaceKey) -> Result<PathBuf, PortError> {
        self.inner.resolve_key(key).await
    }
    async fn cleanup_stale_allocations(
        &self,
        age: Duration,
    ) -> Result<WorkspaceCleanupReport, PortError> {
        self.inner.cleanup_stale_allocations(age).await
    }
    async fn read_workspace_file_to_string(
        &self,
        key: &WorkspaceKey,
        file: &str,
        max: u64,
    ) -> Result<String, PortError> {
        self.inner
            .read_workspace_file_to_string(key, file, max)
            .await
    }
    async fn resolve_child_path(
        &self,
        key: &WorkspaceKey,
        file: &str,
    ) -> Result<PathBuf, PortError> {
        self.inner.resolve_child_path(key, file).await
    }
}
