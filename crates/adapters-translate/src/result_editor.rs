use crate::format_inspector::read_bounded;
use crate::run_plan::DocumentPlan;
use async_trait::async_trait;
use auralis_translation::{
    BlockPolicy, LanguageCode, LanguagePair, ResultId, RunId, SegmentId, SourceHash, TranslationId,
};
use auralis_translation_formats::srt::SrtParsePolicy;
use auralis_translation_formats::vtt::VttParsePolicy;
use auralis_translation_sqlite::{DbError, EditSpec, SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_result_edit::{TranslationResultEdit, TranslationResultEditorPort};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TranslateResultEditor {
    database_path: PathBuf,
}

impl TranslateResultEditor {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationResultEditorPort for TranslateResultEditor {
    async fn commit_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        edit: TranslationResultEdit,
    ) -> Result<Uuid, PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        let intent = intent.clone();
        tokio::task::spawn_blocking(move || {
            commit_edit(&database_path, &source_path, &link, &intent, edit)
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn commit_edit(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    intent: &TranslationRunIntent,
    edit: TranslationResultEdit,
) -> Result<Uuid, PortError> {
    if link.translation_id != intent.translation_id
        || link
            .active_run_id
            .is_some_and(|active| active != intent.run_id)
    {
        return Err(conflict("project link and frozen run intent differ"));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| invalid("translation ID is nil"))?;
    let run_id = RunId::new(intent.run_id).ok_or_else(|| invalid("run ID is nil"))?;
    let base_result_id =
        ResultId::new(edit.base_result_id).ok_or_else(|| invalid("base result ID is nil"))?;
    let result_id = ResultId::new(edit.result_id).ok_or_else(|| invalid("result ID is nil"))?;
    let segment_id =
        SegmentId::new(edit.segment_id).ok_or_else(|| invalid("segment ID must be positive"))?;
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
    if !database_path.is_file() {
        return Err(PortError::NotFound {
            resource: "Translate database".into(),
        });
    }
    let mut database = TranslateDb::open(database_path, SqliteConfig::default())
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
            "stored Translate run differs from frozen project intent",
        ));
    }
    let base = database
        .result(base_result_id)
        .map_err(|error| storage("read_translate_base_result", error))?;
    if base.run_id != run_id || base.source_hash != plan.source_hash() {
        return Err(conflict("base result does not belong to the active run"));
    }
    let committed = database
        .commit_edit(
            &EditSpec {
                base_result_id,
                result_id,
                segment_id,
                lines: edit.lines,
            },
            &plan,
        )
        .map_err(map_edit_error)?;
    Ok(committed.result_id.get())
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation result edit".into(),
        message: message.into(),
    }
}

fn map_edit_error(error: DbError) -> PortError {
    match error {
        DbError::Conflict(message) => conflict(message),
        DbError::InvalidSpec(message) => invalid(message),
        DbError::Verification(message) => invalid(&message),
        other => storage("commit_translate_result_edit", other),
    }
}

fn storage(operation: &'static str, error: DbError) -> PortError {
    PortError::Storage {
        operation,
        message: error.to_string(),
    }
}
