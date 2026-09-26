use ports::error::PortError;
use ports::translation_edit_intents::TranslationEditIntent;

pub(super) const MAX_LINK_REVISION: u64 = 9_007_199_254_740_991;
pub(super) const MAX_RECOVERY_PAGE: u32 = 100;

pub(super) fn validate(intent: &TranslationEditIntent) -> Result<(), PortError> {
    if [
        intent.translation_id,
        intent.result_id,
        intent.base_result_id,
        intent.run_id,
        intent.expected_head_result_id,
    ]
    .iter()
    .any(uuid::Uuid::is_nil)
        || intent.result_id == intent.base_result_id
        || intent.result_id == intent.expected_head_result_id
        || intent.expected_link_revision == 0
        || intent.expected_link_revision >= MAX_LINK_REVISION
        || intent.request_sha256.len() != 64
        || !intent
            .request_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PortError::InvalidSource {
            message: "invalid translation edit intent".into(),
        });
    }
    Ok(())
}

pub(super) fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "TranslationEditIntent".into(),
        message: message.into(),
    }
}
