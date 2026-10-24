use super::super::{control::WorkControl, error};
use ports::{PortError, speech_audio::SpeechAudioLayout};
use serde_json::Value;

pub(super) fn verify(
    value: &Value,
    layout: &SpeechAudioLayout,
    control: &WorkControl,
) -> Result<(), PortError> {
    if value.get("version").and_then(Value::as_u64) != Some(4)
        || value.get("audio_assembly").and_then(Value::as_str)
            != Some(layout.assembly_policy.as_str())
        || value.get("output_sha256").and_then(Value::as_str)
            != Some(layout.aggregate_sha256.as_str())
        || value.get("sample_rate").and_then(Value::as_u64) != Some(48000)
    {
        return Err(error::invalid(
            "Ready raw manifest identity differs from its verified layout",
        ));
    }
    let ids = value
        .get("speech_plan")
        .and_then(|plan| plan.get("expected_utterance_ids"))
        .and_then(Value::as_array)
        .ok_or_else(|| error::invalid("Ready raw manifest has no ordered utterance coverage"))?;
    let segments = value
        .get("segments")
        .and_then(Value::as_array)
        .ok_or_else(|| error::invalid("Ready raw manifest has no technical chunks"))?;
    if ids.len() != layout.utterances.len() || segments.len() > 8192 {
        return Err(error::invalid(
            "Ready raw manifest coverage exceeds or differs from its layout",
        ));
    }
    let mut cursor = 0;
    for (range, expected) in layout.utterances.iter().zip(ids) {
        control.check()?;
        if expected.as_str() != Some(range.utterance_id.as_str()) {
            return Err(error::invalid("Ready raw manifest utterance order changed"));
        }
        for chunk in &range.chunk_ids {
            control.check()?;
            let segment = segments
                .get(cursor)
                .ok_or_else(|| error::invalid("Ready raw manifest omits an utterance chunk"))?;
            if segment.get("utterance_id").and_then(Value::as_str)
                != Some(range.utterance_id.as_str())
                || segment.get("chunk_id").and_then(Value::as_str) != Some(chunk.as_str())
                || segment.get("kind").and_then(Value::as_str) != Some("text")
                || !matches!(
                    segment.get("status").and_then(Value::as_str),
                    Some("generated" | "cached" | "reused_in_job")
                )
            {
                return Err(error::invalid(
                    "Ready raw manifest technical chunk identity or order changed",
                ));
            }
            cursor += 1;
        }
    }
    if cursor != segments.len() {
        return Err(error::invalid("Ready raw manifest contains extra chunks"));
    }
    Ok(())
}
