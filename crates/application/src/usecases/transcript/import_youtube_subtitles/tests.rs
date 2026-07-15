use super::*;
use async_trait::async_trait;
use domain::media::{Artifact, ArtifactKind, MediaSource, SubtitleTrack};
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};
use ports::storage::{ArtifactStore, StagedArtifact};
use ports::workspace::{TempWorkspacePort, WorkspaceAllocation, WorkspaceCleanupReport};
use std::sync::Arc;
use std::sync::Mutex;

use crate::error::ApplicationError;
use crate::test_utils::MockStorageUnitOfWork;

// Mock components
struct MockProjectRepo {
    project: Option<Project>,
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

struct MockSubtitleSource {
    fail_download: bool,
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

struct MockArtifactStoreForSubs {
    fail_delete: bool,
    deleted_keys: Arc<Mutex<Vec<String>>>,
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

struct MockWorkspacePortForSubs {
    fail_delete: bool,
    deleted_keys: Arc<Mutex<Vec<String>>>,
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
        Ok(())
    }
    async fn resolve_key(
        &self,
        _key: &domain::outbox::WorkspaceKey,
    ) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
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
}

#[tokio::test]
async fn test_rollback_on_transaction_failure() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: false,
            deleted_keys: deleted_staging.clone(),
        }),
        Arc::new(MockStorageUnitOfWork::with_failure()), // Will fail commit
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: false,
            deleted_keys: deleted_workspace.clone(),
        }),
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };
    assert!(matches!(err, ApplicationError::Port(_)));

    // Ensure both staging and workspace are deleted
    assert_eq!(deleted_staging.lock().unwrap().len(), 1);
    assert_eq!(deleted_staging.lock().unwrap()[0], "staging_key");
    assert_eq!(deleted_workspace.lock().unwrap().len(), 1);
    assert_eq!(deleted_workspace.lock().unwrap()[0], "tmp/1/subs");
}

#[tokio::test]
async fn test_workspace_error_composites_with_primary_error() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let project_id = project.id().clone();

    let deleted_staging = Arc::new(Mutex::new(vec![]));
    let deleted_workspace = Arc::new(Mutex::new(vec![]));

    let usecase = ImportYoutubeSubtitlesUseCase::new(
        Arc::new(MockProjectRepo {
            project: Some(project),
        }),
        Arc::new(MockSubtitleSource {
            fail_download: false,
        }),
        Arc::new(MockArtifactStoreForSubs {
            fail_delete: true,
            deleted_keys: deleted_staging.clone(),
        }), // staging delete fails
        Arc::new(MockStorageUnitOfWork::with_failure()), // Will fail commit
        Arc::new(MockWorkspacePortForSubs {
            fail_delete: true,
            deleted_keys: deleted_workspace.clone(),
        }), // workspace delete fails
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
        })
        .await;

    assert!(result.is_err());
    let err = match result {
        Ok(_) => panic!("Expected err"),
        Err(e) => e,
    };

    if let ApplicationError::OperationFailedWithCleanup {
        primary,
        cleanup_report,
    } = err
    {
        assert!(matches!(*primary, ApplicationError::Port(_))); // The initial transaction failure
        assert_eq!(cleanup_report.failures.len(), 2);

        // Ensure both failures are reported
        let has_workspace_err = cleanup_report
            .failures
            .iter()
            .any(|f| matches!(f.target, crate::error::CleanupTarget::Workspace { .. }));
        let has_staging_err = cleanup_report
            .failures
            .iter()
            .any(|f| matches!(f.target, crate::error::CleanupTarget::Staging { .. }));
        assert!(has_workspace_err);
        assert!(has_staging_err);
    } else {
        panic!("Expected OperationFailedWithCleanup, got {:?}", err);
    }
}
