use crate::run_plan::DocumentPlan;
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, RunId, SourceHash, TranslationId,
};
use auralis_translation_sqlite::{DbError, TranslateDb};
use ports::error::PortError;
use ports::translation_links::{TranslationLink, TranslationRunIntent};

pub(crate) fn verify(
    database: &TranslateDb,
    source: &[u8],
    link: &TranslationLink,
    intent: &TranslationRunIntent,
) -> Result<DocumentPlan, PortError> {
    if link.translation_id != intent.translation_id {
        return Err(conflict("project link and frozen run intent differ"));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| invalid("translation ID is nil"))?;
    let run_id = RunId::new(intent.run_id).ok_or_else(|| invalid("run ID is nil"))?;
    let pair = match (link.source_language.as_str(), link.target_language.as_str()) {
        ("zh", "ru") => LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian),
        ("ja", "ru") => LanguagePair::new(LanguageCode::Japanese, LanguageCode::Russian),
        _ => return Err(invalid("unsupported translation language pair")),
    }
    .map_err(|error| invalid(&error.to_string()))?;
    if SourceHash::digest(intent.profile_manifest_json.as_bytes()).to_string()
        != intent.profile_sha256
    {
        return Err(conflict("frozen profile bytes differ from their digest"));
    }
    let policy = BlockPolicy::with_context(
        intent.max_target_segments as usize,
        intent.context_before_segments as usize,
        intent.context_after_segments as usize,
    )
    .ok_or_else(|| invalid("invalid frozen block policy"))?;
    let plan = DocumentPlan::new(
        &link.source_format,
        source,
        translation_id,
        run_id,
        pair,
        policy,
    )?;
    if plan.source_hash().to_string() != link.source_sha256
        || plan.parser_version() != intent.parser_version
        || plan.policy_fingerprint(policy).to_string() != intent.policy_fingerprint
    {
        return Err(conflict(
            "original or parser policy differs from frozen run",
        ));
    }
    let translation = database
        .translation(translation_id)
        .map_err(|error| storage("read_translate_translation", error))?;
    if translation.project_id.as_deref() != Some(link.project_id.to_string().as_str())
        || translation.source_artifact_id.as_deref()
            != Some(link.source_artifact_id.to_string().as_str())
        || translation.source_hash != plan.source_hash()
        || translation.source_format != link.source_format
        || translation.language_pair != pair
    {
        return Err(conflict(
            "stored Translate source differs from project link",
        ));
    }
    let run = database
        .run(run_id)
        .map_err(|error| storage("read_translate_run", error))?;
    if run.translation_id != translation_id
        || run.source_hash != plan.source_hash()
        || run.profile_fingerprint != intent.profile_sha256
        || run.parser_version != intent.parser_version
        || run.policy_fingerprint != intent.policy_fingerprint
        || run.glossary_revision.is_some()
        || run.blocks != plan.blocks()
    {
        return Err(conflict(
            "stored Translate run differs from frozen project intent",
        ));
    }
    Ok(plan)
}
fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "frozen translation run".into(),
        message: message.into(),
    }
}
fn storage(operation: &'static str, error: DbError) -> PortError {
    PortError::Storage {
        operation,
        message: error.to_string(),
    }
}
