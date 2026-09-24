use crate::format_inspector::read_bounded;
use async_trait::async_trait;
use auralis_translation::{ResultId, ReviewState, RunId, SourceHash, TranslationId};
use auralis_translation_formats::srt::{SegmentTranslation, SrtDocument, SrtParsePolicy};
use auralis_translation_formats::vtt::{VttDocument, VttParsePolicy};
use auralis_translation_sqlite::{SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::TranslationLink;
use ports::translation_result::{TranslationResultPort, VerifiedTranslationResult};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct TranslateResultReader {
    database_path: PathBuf,
}

impl TranslateResultReader {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }
}

#[async_trait]
impl TranslationResultPort for TranslateResultReader {
    async fn verified_result(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        expected_run_id: Uuid,
        result_id: Uuid,
    ) -> Result<VerifiedTranslationResult, PortError> {
        let database_path = self.database_path.clone();
        let source_path = source_path.to_path_buf();
        let link = link.clone();
        tokio::task::spawn_blocking(move || {
            read_verified(
                &database_path,
                &source_path,
                &link,
                expected_run_id,
                result_id,
            )
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
    }
}

fn read_verified(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    expected_run: Uuid,
    result_id: Uuid,
) -> Result<VerifiedTranslationResult, PortError> {
    if link
        .active_run_id
        .is_some_and(|active| active != expected_run)
    {
        return Err(conflict("requested result differs from active project run"));
    }
    let max_bytes = match link.source_format.as_str() {
        "srt" => SrtParsePolicy::default().max_bytes(),
        "vtt" => VttParsePolicy::default().max_bytes(),
        _ => {
            return Err(PortError::Unsupported {
                message: "unsupported translation source format".into(),
            });
        }
    };
    let source = read_bounded(source_path, max_bytes)?;
    let source_hash = SourceHash::digest(&source);
    if source_hash.to_string() != link.source_sha256 {
        return Err(conflict("original subtitle bytes changed"));
    }
    let translation_id =
        TranslationId::new(link.translation_id).ok_or_else(|| conflict("translation ID is nil"))?;
    let run_id = RunId::new(expected_run).ok_or_else(|| conflict("run ID is nil"))?;
    let result_id = ResultId::new(result_id).ok_or_else(|| conflict("result ID is nil"))?;
    let database = TranslateDb::open(database_path, SqliteConfig::default())
        .map_err(|error| storage("open_translate_database", error))?;
    let translation = database
        .translation(translation_id)
        .map_err(|error| storage("read_translate_translation", error))?;
    if translation.project_id.as_deref() != Some(link.project_id.to_string().as_str())
        || translation.source_artifact_id.as_deref()
            != Some(link.source_artifact_id.to_string().as_str())
        || translation.source_hash != source_hash
        || translation.source_format != link.source_format
    {
        return Err(conflict(
            "Translate source identity differs from project link",
        ));
    }
    let run = database
        .run(run_id)
        .map_err(|error| storage("read_translate_run", error))?;
    let result = database
        .result(result_id)
        .map_err(|error| storage("read_translate_result", error))?;
    if run.translation_id != translation_id
        || run.source_hash != source_hash
        || result.run_id != run_id
        || result.source_hash != source_hash
    {
        return Err(conflict(
            "result does not belong to the active source and run",
        ));
    }
    let replacements = result
        .selected
        .iter()
        .map(|segment| SegmentTranslation {
            id: segment.id,
            lines: segment.lines.clone(),
        })
        .collect::<Vec<_>>();
    let output_bytes = match link.source_format.as_str() {
        "srt" => SrtDocument::parse(&source)
            .and_then(|document| document.render(&replacements))
            .map_err(|error| invalid(&error.to_string()))?,
        "vtt" => VttDocument::parse(&source)
            .and_then(|document| document.render(&replacements))
            .map_err(|error| invalid(&error.to_string()))?,
        _ => unreachable!(),
    };
    let output_sha256 = SourceHash::digest(&output_bytes).to_string();
    if output_sha256 != result.output_hash.to_string() {
        return Err(conflict(
            "reconstructed result digest differs from stored result",
        ));
    }
    Ok(VerifiedTranslationResult {
        result_id: result_id.get(),
        run_id: run_id.get(),
        output_sha256,
        output_bytes,
        needs_review: result.review_state == ReviewState::NeedsReview,
    })
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "translation result".into(),
        message: message.into(),
    }
}

fn storage(operation: &'static str, error: auralis_translation_sqlite::DbError) -> PortError {
    PortError::Storage {
        operation,
        message: error.to_string(),
    }
}
