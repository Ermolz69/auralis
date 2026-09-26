use crate::error::ApplicationError;
use crate::usecases::translation::edit_fingerprint;
use ports::error::PortError;
use ports::translation_edit_intents::TranslationEditIntent;
use ports::translation_result::VerifiedTranslationResult;

pub(super) fn verify(
    intent: &TranslationEditIntent,
    result: &VerifiedTranslationResult,
) -> Result<(), ApplicationError> {
    let origin = result
        .edit_provenance
        .as_ref()
        .ok_or_else(|| conflict("Historical edit has no persisted provenance"))?;
    if result.result_id != intent.result_id
        || result.run_id != intent.run_id
        || !result.needs_review
        || origin.base_result_id != intent.base_result_id
        || origin.observed_head_result_id != intent.expected_head_result_id
    {
        return Err(conflict(
            "Historical result differs from its admitted ancestry",
        ));
    }
    let segment = result
        .segments
        .iter()
        .find(|segment| segment.id == origin.segment_id)
        .ok_or_else(|| conflict("Edited segment is absent from the verified result"))?;
    if edit_fingerprint::digest(intent, origin.segment_id, &segment.translated_lines)
        != intent.request_sha256
    {
        return Err(conflict(
            "Historical result differs from its admitted request fingerprint",
        ));
    }
    Ok(())
}

fn conflict(message: &str) -> ApplicationError {
    PortError::Conflict {
        resource: "historical translation publication".into(),
        message: message.into(),
    }
    .into()
}
