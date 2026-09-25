pub mod begin;
pub mod comparison;
pub mod edit;
pub mod import_source;
pub mod inspect;
pub mod list;
pub mod pause;
pub mod publish;
pub mod status;

pub(super) fn parse_translation_id(
    id: &str,
) -> Result<uuid::Uuid, crate::dto::error::CommandError> {
    uuid::Uuid::parse_str(id)
        .map_err(|_| crate::dto::error::CommandError::Validation("Invalid identifier".into()))
}
