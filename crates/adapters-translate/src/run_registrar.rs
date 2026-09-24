use crate::format_inspector::read_bounded;
use crate::run_plan::DocumentPlan;
use async_trait::async_trait;
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, RunId, SourceHash, TranslationId,
};
use auralis_translation_formats::srt::SrtParsePolicy;
use auralis_translation_formats::vtt::VttParsePolicy;
use auralis_translation_llamacpp::ModelProfile;
use auralis_translation_sqlite::{RunSpec, SqliteConfig, TranslateDb, TranslationSpec};
use chrono::{DateTime, Utc};
use ports::error::PortError;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_FROZEN_PROFILE_BYTES: usize = 65_536;

pub struct TranslateRunRegistrar {
    database_path: PathBuf,
}

impl TranslateRunRegistrar {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationRunRegistrationPort for TranslateRunRegistrar {
    fn prepare_intent(
        &self,
        format: TranslationSourceFormat,
        translation_id: Uuid,
        run_id: Uuid,
        profile_manifest_json: String,
        created_at: DateTime<Utc>,
    ) -> Result<TranslationRunIntent, PortError> {
        if translation_id.is_nil()
            || run_id.is_nil()
            || profile_manifest_json.len() > MAX_FROZEN_PROFILE_BYTES
        {
            return Err(invalid("invalid frozen run identity or profile size"));
        }
        let profile = ModelProfile::from_json(profile_manifest_json.as_bytes())
            .map_err(|error| invalid(&error.to_string()))?;
        let policy = BlockPolicy::with_context(
            profile.target_segments_per_block,
            profile.context_before_segments,
            profile.context_after_segments,
        )
        .ok_or_else(|| invalid("invalid block policy"))?;
        let (parser_version, policy_fingerprint) = match format {
            TranslationSourceFormat::Srt => (
                auralis_translation_formats::srt::SrtRunPlan::PARSER_VERSION,
                auralis_translation_formats::srt::SrtRunPlan::policy_fingerprint(policy),
            ),
            TranslationSourceFormat::Vtt => (
                auralis_translation_formats::vtt::VttRunPlan::PARSER_VERSION,
                auralis_translation_formats::vtt::VttRunPlan::policy_fingerprint(policy),
            ),
        };
        Ok(TranslationRunIntent {
            run_id,
            translation_id,
            profile_sha256: SourceHash::digest(profile_manifest_json.as_bytes()).to_string(),
            profile_manifest_json,
            max_target_segments: policy.max_target_segments() as u32,
            context_before_segments: policy.context_before_segments() as u32,
            context_after_segments: policy.context_after_segments() as u32,
            parser_version,
            policy_fingerprint: policy_fingerprint.to_string(),
            created_at,
        })
    }

    async fn ensure_run(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
    ) -> Result<(), PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        let intent = intent.clone();
        tokio::task::spawn_blocking(move || {
            ensure_run(&database_path, &source_path, &link, &intent)
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn ensure_run(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    intent: &TranslationRunIntent,
) -> Result<(), PortError> {
    if link.translation_id != intent.translation_id || link.active_run_id != Some(intent.run_id) {
        return Err(conflict(
            "run intent does not match the active project link",
        ));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| conflict("translation ID is nil"))?;
    let run_id = RunId::new(intent.run_id).ok_or_else(|| conflict("run ID is nil"))?;
    let source_language = match link.source_language.as_str() {
        "zh" => LanguageCode::Chinese,
        "ja" => LanguageCode::Japanese,
        _ => return Err(invalid("unsupported source language")),
    };
    if link.target_language != "ru" {
        return Err(invalid("unsupported target language"));
    }
    let pair = LanguagePair::new(source_language, LanguageCode::Russian)
        .map_err(|error| invalid(&error.to_string()))?;
    let profile_bytes = intent.profile_manifest_json.as_bytes();
    if SourceHash::digest(profile_bytes).to_string() != intent.profile_sha256 {
        return Err(conflict("frozen profile hash differs from its bytes"));
    }
    let profile =
        ModelProfile::from_json(profile_bytes).map_err(|error| invalid(&error.to_string()))?;
    if profile.target_segments_per_block != intent.max_target_segments as usize
        || profile.context_before_segments != intent.context_before_segments as usize
        || profile.context_after_segments != intent.context_after_segments as usize
    {
        return Err(conflict("frozen profile and block policy differ"));
    }
    let policy = BlockPolicy::with_context(
        intent.max_target_segments as usize,
        intent.context_before_segments as usize,
        intent.context_after_segments as usize,
    )
    .ok_or_else(|| invalid("invalid block policy"))?;
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
    if plan.source_hash().to_string() != link.source_sha256 {
        return Err(conflict("original subtitle bytes changed"));
    }
    if plan.parser_version() != intent.parser_version
        || plan.policy_fingerprint(policy).to_string() != intent.policy_fingerprint
    {
        return Err(conflict(
            "parser or source policy differs from frozen intent",
        ));
    }
    let segments = plan.snapshot()?;
    let source_len = u64::try_from(source.len()).map_err(|error| invalid(&error.to_string()))?;
    let mut database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| database_error("open_translate_database", error))?;
    database
        .ensure_translation(&TranslationSpec {
            translation_id,
            project_id: Some(link.project_id.to_string()),
            source_artifact_id: Some(link.source_artifact_id.to_string()),
            source_locator: None,
            source_hash: plan.source_hash(),
            source_format: link.source_format.clone(),
            language_pair: pair,
        })
        .map_err(|error| database_error("ensure_translation", error))?;
    database
        .ensure_segments(translation_id, source_len, &segments)
        .map_err(|error| database_error("ensure_segments", error))?;
    database
        .ensure_run(&RunSpec {
            run_id,
            translation_id,
            source_hash: plan.source_hash(),
            profile_fingerprint: intent.profile_sha256.clone(),
            parser_version: intent.parser_version,
            policy_fingerprint: intent.policy_fingerprint.clone(),
            glossary_revision: None,
            blocks: plan.blocks().to_vec(),
        })
        .map_err(|error| database_error("ensure_run", error))?;
    Ok(())
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation run".into(),
        message: message.into(),
    }
}

fn database_error(
    operation: &'static str,
    error: auralis_translation_sqlite::DbError,
) -> PortError {
    match error {
        auralis_translation_sqlite::DbError::Conflict(reason) => conflict(reason),
        other => PortError::Storage {
            operation,
            message: other.to_string(),
        },
    }
}
