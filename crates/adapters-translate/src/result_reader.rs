use crate::format_inspector::read_bounded;
use async_trait::async_trait;
use auralis_translation::{ResultId, ReviewState, RunId, SourceHash, TranslationId};
use auralis_translation_formats::srt::{SegmentTranslation, SrtDocument, SrtParsePolicy};
use auralis_translation_formats::vtt::{VttDocument, VttParsePolicy};
use auralis_translation_sqlite::{SqliteConfig, TranslateDb};
use ports::error::PortError;
use ports::translation_links::TranslationLink;
use ports::translation_result::{
    ComparedTranslationSegment, MAX_COMPARISON_PAGE_SIZE, TranslationComparisonPage,
    TranslationResultPort, VerifiedTranslationResult,
};
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

    async fn comparison_page(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        expected_run_id: Uuid,
        result_id: Uuid,
        offset: u32,
        limit: u32,
    ) -> Result<TranslationComparisonPage, PortError> {
        if !(1..=MAX_COMPARISON_PAGE_SIZE).contains(&limit) {
            return Err(invalid(
                "comparison page size is outside the supported range",
            ));
        }
        let verified = self
            .verified_result(source_path, link, expected_run_id, result_id)
            .await?;
        let total_segments = u32::try_from(verified.segments.len())
            .map_err(|_| invalid("translation has too many segments to compare"))?;
        let segments = verified
            .segments
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
        Ok(TranslationComparisonPage {
            result_id: verified.result_id,
            run_id: verified.run_id,
            needs_review: verified.needs_review,
            total_segments,
            offset,
            segments,
        })
    }
}

fn read_verified(
    database_path: &Path,
    source_path: &Path,
    link: &TranslationLink,
    expected_run: Uuid,
    result_id: Uuid,
) -> Result<VerifiedTranslationResult, PortError> {
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
            "result does not belong to the linked source and requested run",
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
    let sources = database
        .segments(translation_id)
        .map_err(|error| storage("read_translate_segments", error))?;
    if sources.len() != result.selected.len() {
        return Err(conflict(
            "result selection differs from stored source segments",
        ));
    }
    let segments = sources
        .into_iter()
        .zip(result.selected)
        .map(|(source, translated)| {
            if source.id != translated.id || source.source_lines.len() != translated.lines.len() {
                return Err(conflict("result segment differs from source mapping"));
            }
            Ok(ComparedTranslationSegment {
                id: source.id.get(),
                ordinal: source.ordinal,
                cue_label: source.cue_label,
                start_ms: source.start_ms,
                end_ms: source.end_ms,
                source_lines: source.source_lines,
                translated_lines: translated.lines,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(VerifiedTranslationResult {
        result_id: result_id.get(),
        run_id: run_id.get(),
        result_revision: result.revision,
        output_sha256,
        output_bytes,
        needs_review: result.review_state == ReviewState::NeedsReview,
        segments,
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
