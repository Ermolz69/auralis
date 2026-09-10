use crate::error::ApplicationError;
use crate::usecases::import_cleanup::cleanup_staging_and_workspace;
use domain::outbox::WorkspaceKey;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

pub(super) async fn cleanup_failed_import(
    primary: ApplicationError,
    staging_key: Option<&str>,
    workspace_key: &WorkspaceKey,
    store: &impl ArtifactStore,
    workspace: &dyn TempWorkspacePort,
) -> ApplicationError {
    cleanup_staging_and_workspace(store, staging_key, workspace, workspace_key)
        .await
        .into_error(primary)
}
