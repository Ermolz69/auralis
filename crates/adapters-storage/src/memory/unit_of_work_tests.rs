#![allow(clippy::unwrap_used)]

use super::{InMemoryDatabase, InMemoryStorageUnitOfWork};
use async_trait::async_trait;
use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, ArtifactState};
use domain::project::ProjectId;
use ports::artifact_index::ArtifactIndex;
use ports::error::PortError;
use ports::storage::{ArtifactStore, StagedArtifact};
use ports::transaction::{CommitStagedArtifactWrite, StorageUnitOfWork};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct CountingArtifactStore {
    finalize_count: AtomicUsize,
}

#[async_trait]
impl ArtifactStore for CountingArtifactStore {
    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }

    async fn import_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        self.finalize_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn resolve_artifact(&self, _artifact: &Artifact) -> Result<PathBuf, PortError> {
        unreachable!()
    }

    async fn delete_storage_key(&self, _storage_key: &str) -> Result<(), PortError> {
        unreachable!()
    }

    async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
        unreachable!()
    }

    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        unreachable!()
    }

    async fn cleanup_stale_staging(&self, _max_age: std::time::Duration) -> Result<(), PortError> {
        unreachable!()
    }

    async fn stage_owned_workspace_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _workspace_port: &dyn ports::workspace::TempWorkspacePort,
        _allocation_key: &domain::outbox::WorkspaceKey,
        _relative_file: &str,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }
}

struct NoopArtifactIndex;

#[async_trait]
impl ArtifactIndex for NoopArtifactIndex {
    async fn add(&self, _project_id: &ProjectId, _artifact: &Artifact) -> Result<(), PortError> {
        Ok(())
    }

    async fn get(&self, _id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
        Ok(None)
    }

    async fn check_exists(&self, _id: &ArtifactId) -> Result<bool, PortError> {
        Ok(false)
    }

    async fn list_by_project(&self, _project_id: &ProjectId) -> Result<Vec<Artifact>, PortError> {
        Ok(vec![])
    }

    async fn list_by_project_and_kind(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError> {
        Ok(vec![])
    }

    async fn delete(&self, _id: &ArtifactId) -> Result<(), PortError> {
        Ok(())
    }

    async fn update_state(
        &self,
        _id: &ArtifactId,
        _state: ArtifactState,
        _ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
    ) -> Result<(), PortError> {
        Ok(())
    }
}

#[tokio::test]
async fn staged_artifact_write_validates_before_test_file_effect() {
    let store = Arc::new(CountingArtifactStore {
        finalize_count: AtomicUsize::new(0),
    });
    let uow = InMemoryStorageUnitOfWork::new(
        Arc::new(Mutex::new(InMemoryDatabase::new())),
        Arc::new(NoopArtifactIndex),
        store.clone(),
    );
    let project_id = ProjectId::new();
    let artifact_id = ArtifactId::new();
    let artifact = Artifact {
        id: artifact_id,
        kind: ArtifactKind::SourceVideo,
        location: ArtifactLocation::StorageKey("other/final.mp4".to_string()),
        size_bytes: Some(1),
        state: ArtifactState::PendingFinalize,
        created_at: domain::chrono::Utc::now(),
        updated_at: domain::chrono::Utc::now(),
        ready_at: None,
    };

    let result = uow
        .commit_staged_artifact_write(CommitStagedArtifactWrite {
            project_id: project_id.clone(),
            artifact,
            staging_key: format!(".staging/{}/file.mp4", project_id),
            final_key: format!("{}/source-video/file.mp4", project_id),
            temp_workspace_key: None,
        })
        .await;

    assert!(matches!(result, Err(PortError::Unexpected { .. })));
    assert_eq!(store.finalize_count.load(Ordering::SeqCst), 0);
}
