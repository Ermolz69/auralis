use crate::error::ApplicationError;
use ports::translation_links::{
    TranslationLink, TranslationLinkStore, TranslationPublicationState,
};
use uuid::Uuid;

pub(super) async fn resolve(
    links: &dyn TranslationLinkStore,
    link: &TranslationLink,
) -> Result<Uuid, ApplicationError> {
    if let Some(run_id) = link.active_run_id {
        return Ok(run_id);
    }
    let selected = link
        .selected_result_id
        .ok_or_else(|| ApplicationError::InvalidOperation {
            message: "Translation has no active or selected result".into(),
        })?;
    let publication = links
        .get_publication(link.translation_id, selected)
        .await?
        .ok_or_else(|| ApplicationError::InvalidOperation {
            message: "Selected translation has no publication".into(),
        })?;
    if publication.source_sha256 != link.source_sha256
        || publication.state != TranslationPublicationState::Ready
    {
        return Err(ApplicationError::InvalidOperation {
            message: "Selected publication does not match the source".into(),
        });
    }
    Ok(publication.run_id)
}
