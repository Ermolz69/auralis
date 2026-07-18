#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use std::sync::Mutex;

use domain::media::MediaSource;
use domain::project::Project;

use super::mocks::{
    MockArtifactStoreForSubs, MockProjectRepo, MockSubtitleSource, MockWorkspacePortForSubs,
};
use super::*;
use crate::error::ApplicationError;
use crate::test_utils::MockStorageUnitOfWork;

#[tokio::test]
async fn test_rollback_on_transaction_failure() {
    let mut project = Project::new("Test".into());
    project
        .import_source(MediaSource::YoutubeUrl { url: "test".into() }, None)
        .unwrap();
    let job_id = domain::job::JobId::new();
    project.mark_ready_for_processing().unwrap();
    project.start_processing(job_id.clone()).unwrap();
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
            allocated_path: Arc::new(Mutex::new(None)),
        }),
    );

    let result = usecase
        .execute(ImportYoutubeSubtitlesRequest {
            project_id,
            preferred_languages: vec!["en".into()],
            allow_auto_generated: false,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            job_id,
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
