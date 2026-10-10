pub mod begin;
pub mod comparison;
pub mod edit;
pub mod edit_context;
pub mod export;
pub mod historical_edit;
pub mod history;
pub mod import_source;
pub mod inspect;
pub mod list;
mod package_admission;
mod package_error;
pub mod package_install;
pub mod package_select;
pub mod package_status;
pub mod pause;
pub mod publication_view;
pub mod publish;
pub mod review;
pub mod select_result;
pub mod start;
pub mod status;

pub(super) fn parse_translation_id(
    id: &str,
) -> Result<uuid::Uuid, crate::dto::error::CommandError> {
    uuid::Uuid::parse_str(id)
        .map_err(|_| crate::dto::error::CommandError::Validation("Invalid identifier".into()))
}
