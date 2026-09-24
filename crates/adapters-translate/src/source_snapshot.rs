use crate::run_plan::DocumentPlan;
use auralis_translation::SegmentId;
use auralis_translation_formats::srt::TextSlot;
use auralis_translation_sqlite::SegmentSpec;
use ports::error::PortError;

pub(crate) fn from_plan(plan: &DocumentPlan) -> Result<Vec<SegmentSpec>, PortError> {
    match plan {
        DocumentPlan::Srt(plan) => plan
            .source_segments()
            .iter()
            .enumerate()
            .map(|(ordinal, segment)| {
                map_segment(
                    segment.id,
                    ordinal,
                    Some(segment.cue_label.clone()),
                    segment.start_ms,
                    segment.end_ms,
                    &segment.text_slots,
                    auralis_translation_formats::srt::SrtRunPlan::PARSER_VERSION,
                )
            })
            .collect(),
        DocumentPlan::Vtt(plan) => plan
            .source_segments()
            .iter()
            .enumerate()
            .map(|(ordinal, segment)| {
                map_segment(
                    segment.id,
                    ordinal,
                    segment.cue_id.clone(),
                    segment.start_ms,
                    segment.end_ms,
                    &segment.text_slots,
                    auralis_translation_formats::vtt::VttRunPlan::PARSER_VERSION,
                )
            })
            .collect(),
    }
}

fn map_segment(
    id: SegmentId,
    ordinal: usize,
    cue_label: Option<String>,
    start_ms: u64,
    end_ms: u64,
    text_slots: &[TextSlot],
    parser_version: u32,
) -> Result<SegmentSpec, PortError> {
    let ordinal = u32::try_from(ordinal).map_err(invalid_integer)?;
    let text_ranges = text_slots
        .iter()
        .map(|slot| {
            Ok(
                u64::try_from(slot.byte_range.start).map_err(invalid_integer)?
                    ..u64::try_from(slot.byte_range.end).map_err(invalid_integer)?,
            )
        })
        .collect::<Result<_, PortError>>()?;
    Ok(SegmentSpec {
        id,
        ordinal,
        cue_label,
        start_ms,
        end_ms,
        source_lines: text_slots.iter().map(|slot| slot.text.clone()).collect(),
        text_ranges,
        parser_version,
    })
}

fn invalid_integer(error: std::num::TryFromIntError) -> PortError {
    PortError::InvalidSource {
        message: error.to_string(),
    }
}
