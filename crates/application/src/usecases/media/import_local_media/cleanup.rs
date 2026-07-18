use crate::error::{ApplicationError, CleanupReport, CleanupTarget};
use ports::storage::ArtifactStore;

pub async fn cleanup_after_stage<S: ArtifactStore>(
    primary: ApplicationError,
    staging_key: &str,
    artifact_store: &S,
) -> ApplicationError {
    match artifact_store.delete_storage_key(staging_key).await {
        Ok(_) => primary,
        Err(e) => {
            let mut report = CleanupReport::new();
            report.add_failure(CleanupTarget::staging(staging_key), e);
            ApplicationError::OperationFailedWithCleanup {
                primary: Box::new(primary),
                cleanup_report: report,
            }
        }
    }
}
