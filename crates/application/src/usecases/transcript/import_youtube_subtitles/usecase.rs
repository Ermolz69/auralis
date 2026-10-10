use super::{
    cleanup::ImportCleanupCoordinator,
    types::{ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesResponse},
    vtt_parser::{parse_vtt, select_best_subtitle_track},
};
use crate::error::ApplicationError;
use ports::{
    repository::ProjectRepository,
    source::{DownloadSubtitleRequest, SubtitleSourcePort},
    storage::ArtifactStore,
    transaction::{CommitTranscriptImport, StorageUnitOfWork},
    workspace::TempWorkspacePort,
};
use std::sync::Arc;

mod commit;
mod execute;

pub struct ImportYoutubeSubtitlesUseCase {
    project_repo: Arc<dyn ProjectRepository>,
    subtitle_source: Arc<dyn SubtitleSourcePort>,
    artifact_store: Arc<dyn ArtifactStore>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    workspace_port: Arc<dyn TempWorkspacePort>,
    cleanup_coordinator: ImportCleanupCoordinator,
    cpu_executor: Option<Arc<dyn ports::job_resources::CpuExecutorPort>>,
    max_subtitle_bytes: u64,
}

impl ImportYoutubeSubtitlesUseCase {
    pub fn new(
        project_repo: Arc<dyn ProjectRepository>,
        subtitle_source: Arc<dyn SubtitleSourcePort>,
        artifact_store: Arc<dyn ArtifactStore>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        workspace_port: Arc<dyn TempWorkspacePort>,
    ) -> Self {
        Self {
            project_repo,
            subtitle_source,
            artifact_store: artifact_store.clone(),
            storage_uow,
            workspace_port: workspace_port.clone(),
            cleanup_coordinator: ImportCleanupCoordinator::new(artifact_store, workspace_port),
            cpu_executor: None,
            max_subtitle_bytes: 10 * 1024 * 1024,
        }
    }

    pub fn with_cpu_executor(
        mut self,
        executor: Arc<dyn ports::job_resources::CpuExecutorPort>,
        max_subtitle_bytes: u64,
    ) -> Self {
        self.cpu_executor = Some(executor);
        self.max_subtitle_bytes = max_subtitle_bytes;
        self
    }
}
