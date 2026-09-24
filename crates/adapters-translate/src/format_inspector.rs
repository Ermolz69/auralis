use async_trait::async_trait;
use auralis_translation::SourceHash;
use auralis_translation_formats::srt::{SrtDocument, SrtParsePolicy, SrtRunPlan};
use auralis_translation_formats::vtt::{VttDocument, VttParsePolicy, VttRunPlan};
use ports::error::PortError;
use ports::translation_inspection::{
    TranslationInspection, TranslationInspectorPort, TranslationSegment, TranslationSourceFormat,
};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct FormatInspector;

#[async_trait]
impl TranslationInspectorPort for FormatInspector {
    async fn inspect(
        &self,
        source_path: &Path,
        format: TranslationSourceFormat,
    ) -> Result<TranslationInspection, PortError> {
        let path = source_path.to_path_buf();
        tokio::task::spawn_blocking(move || inspect_path(path, format))
            .await
            .map_err(|error| PortError::Unexpected {
                message: error.to_string(),
            })?
    }
}

fn inspect_path(
    path: PathBuf,
    format: TranslationSourceFormat,
) -> Result<TranslationInspection, PortError> {
    let max_bytes = match format {
        TranslationSourceFormat::Srt => SrtParsePolicy::default().max_bytes(),
        TranslationSourceFormat::Vtt => VttParsePolicy::default().max_bytes(),
    };
    let source = read_bounded(&path, max_bytes)?;
    let source_sha256 = SourceHash::digest(&source).to_string();
    let source_size = u64::try_from(source.len()).map_err(|error| PortError::InvalidSource {
        message: error.to_string(),
    })?;
    let (parser_version, segments) = match format {
        TranslationSourceFormat::Srt => {
            let document =
                SrtDocument::parse(&source).map_err(|error| PortError::InvalidSource {
                    message: error.to_string(),
                })?;
            let segments = document
                .segments()
                .iter()
                .enumerate()
                .map(|(ordinal, segment)| {
                    map_segment(
                        segment.id.get(),
                        ordinal,
                        Some(segment.cue_label.clone()),
                        segment.start_ms,
                        segment.end_ms,
                        &segment.text_slots,
                    )
                })
                .collect::<Result<_, _>>()?;
            (SrtRunPlan::PARSER_VERSION, segments)
        }
        TranslationSourceFormat::Vtt => {
            let document =
                VttDocument::parse(&source).map_err(|error| PortError::InvalidSource {
                    message: error.to_string(),
                })?;
            let segments = document
                .segments()
                .iter()
                .enumerate()
                .map(|(ordinal, segment)| {
                    map_segment(
                        segment.id.get(),
                        ordinal,
                        segment.cue_id.clone(),
                        segment.start_ms,
                        segment.end_ms,
                        &segment.text_slots,
                    )
                })
                .collect::<Result<_, _>>()?;
            (VttRunPlan::PARSER_VERSION, segments)
        }
    };
    Ok(TranslationInspection {
        format,
        source_sha256,
        source_size,
        parser_version,
        segments,
    })
}

fn map_segment(
    id: u32,
    ordinal: usize,
    cue_label: Option<String>,
    start_ms: u64,
    end_ms: u64,
    text_slots: &[auralis_translation_formats::srt::TextSlot],
) -> Result<TranslationSegment, PortError> {
    let ordinal = u32::try_from(ordinal).map_err(|error| PortError::InvalidSource {
        message: error.to_string(),
    })?;
    let text_ranges = text_slots
        .iter()
        .map(|slot| {
            Ok(u64::try_from(slot.byte_range.start).map_err(invalid_range)?
                ..u64::try_from(slot.byte_range.end).map_err(invalid_range)?)
        })
        .collect::<Result<_, PortError>>()?;
    Ok(TranslationSegment {
        id,
        ordinal,
        cue_label,
        start_ms,
        end_ms,
        source_lines: text_slots.iter().map(|slot| slot.text.clone()).collect(),
        text_ranges,
    })
}

fn invalid_range(error: std::num::TryFromIntError) -> PortError {
    PortError::InvalidSource {
        message: error.to_string(),
    }
}

fn read_bounded(path: &Path, max_bytes: usize) -> Result<Vec<u8>, PortError> {
    let file = std::fs::File::open(path).map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() > max_bytes as u64 {
        return Err(PortError::InvalidSource {
            message: "subtitle source exceeds the configured size limit".into(),
        });
    }
    let mut source = Vec::new();
    file.take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut source)
        .map_err(io_error)?;
    if source.len() > max_bytes {
        return Err(PortError::InvalidSource {
            message: "subtitle source exceeds the configured size limit".into(),
        });
    }
    Ok(source)
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
