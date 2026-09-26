use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::CommandError;

pub(super) async fn ensure_no_active_jobs(
    usecases: &TranslationUseCases,
) -> Result<(), CommandError> {
    if !usecases
        .jobs
        .list_active()
        .await
        .map_err(CommandError::from)?
        .is_empty()
    {
        return Err(CommandError::Busy(
            "Pause or finish active translation jobs before changing the model".into(),
        ));
    }
    Ok(())
}
