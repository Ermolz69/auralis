use crate::error::{ApplicationError, CleanupReport};
use crate::usecases::import_cleanup::cleanup_staging;
use ports::storage::ArtifactStore;

pub async fn cleanup_after_stage<S: ArtifactStore>(
    primary: ApplicationError,
    staging_key: &str,
    artifact_store: &S,
) -> ApplicationError {
    let mut report = CleanupReport::new();
    cleanup_staging(artifact_store, staging_key, &mut report).await;
    report.into_error(primary)
}
