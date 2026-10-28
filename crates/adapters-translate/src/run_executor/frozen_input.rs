use super::{conflict, invalid, storage};
use crate::format_inspector::read_bounded;
use crate::run_plan::DocumentPlan;
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, RunId, RunState, SourceHash, TranslationId,
};
use auralis_translation_formats::srt::SrtParsePolicy;
use auralis_translation_formats::vtt::VttParsePolicy;
use auralis_translation_llamacpp::ModelProfile;
use auralis_translation_sqlite::{RunSpec, SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use std::path::Path;

pub(super) struct PreparedRun {
    pub(super) database: TranslateDb,
    pub(super) run: RunSpec,
    pub(super) run_id: RunId,
    pub(super) plan: DocumentPlan,
    pub(super) profile: ModelProfile,
    pub(super) state: RunState,
}

pub(super) fn prepare(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    intent: &TranslationRunIntent,
    allow_unchecked_profile: bool,
) -> Result<PreparedRun, PortError> {
    if link.translation_id != intent.translation_id || link.active_run_id != Some(intent.run_id) {
        return Err(conflict("active link and frozen run intent differ"));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| invalid("translation ID is nil"))?;
    let run_id = RunId::new(intent.run_id).ok_or_else(|| invalid("run ID is nil"))?;
    let pair = match (link.source_language.as_str(), link.target_language.as_str()) {
        ("zh", "ru") => LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian),
        ("ja", "ru") => LanguagePair::new(LanguageCode::Japanese, LanguageCode::Russian),
        _ => return Err(invalid("unsupported language pair")),
    }
    .map_err(|error| invalid(&error.to_string()))?;
    let profile_bytes = intent.profile_manifest_json.as_bytes();
    if SourceHash::digest(profile_bytes).to_string() != intent.profile_sha256 {
        return Err(conflict("frozen model profile hash differs from its bytes"));
    }
    let profile =
        ModelProfile::from_json(profile_bytes).map_err(|error| invalid(&error.to_string()))?;
    if profile.model_file_bytes.is_none() && !allow_unchecked_profile {
        return Err(PortError::Unsupported {
            message: "Auralis execution requires a checked model profile".into(),
        });
    }
    let policy = BlockPolicy::with_context(
        intent.max_target_segments as usize,
        intent.context_before_segments as usize,
        intent.context_after_segments as usize,
    )
    .ok_or_else(|| invalid("invalid block policy"))?;
    if profile.target_segments_per_block != policy.max_target_segments()
        || profile.context_before_segments != policy.context_before_segments()
        || profile.context_after_segments != policy.context_after_segments()
    {
        return Err(conflict(
            "profile block policy differs from frozen run intent",
        ));
    }
    let max_bytes = match link.source_format.as_str() {
        "srt" => SrtParsePolicy::default().max_bytes(),
        "vtt" => VttParsePolicy::default().max_bytes(),
        _ => return Err(invalid("unsupported subtitle format")),
    };
    let source = read_bounded(source_path, max_bytes)?;
    let plan = DocumentPlan::new(
        &link.source_format,
        &source,
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
    let database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| storage("open_translate_database", error))?;
    let translation = database
        .translation(translation_id)
        .map_err(|error| storage("read_translate_translation", error))?;
    if translation.project_id.as_deref() != Some(link.project_id.to_string().as_str())
        || translation.source_artifact_id.as_deref()
            != Some(link.source_artifact_id.to_string().as_str())
        || translation.source_hash != plan.source_hash()
        || translation.source_format != link.source_format
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
            "stored Translate run differs from frozen Auralis intent",
        ));
    }
    let state = database
        .run_state(run_id)
        .map_err(|error| storage("read_translate_run_state", error))?;
    Ok(PreparedRun {
        database,
        run,
        run_id,
        plan,
        profile,
        state,
    })
}
