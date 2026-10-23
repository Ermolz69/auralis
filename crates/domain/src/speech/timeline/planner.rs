use super::*;
use crate::speech::{SpeechId, TimingMode};
use std::collections::BTreeSet;

const SUPPORTED_RATES: [u32; 13] = [
    8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
];

fn scale_frames(frames: u64, rate: u32) -> Result<u64, TimelineConflict> {
    let scaled = (u128::from(frames) * u128::from(ASSEMBLY_SAMPLE_RATE)).div_ceil(u128::from(rate));
    u64::try_from(scaled).map_err(|_| TimelineConflict::ArithmeticOverflow)
}

fn milliseconds(value: u64) -> Result<u64, TimelineConflict> {
    value
        .checked_mul(48)
        .ok_or(TimelineConflict::ArithmeticOverflow)
}

/// Pure metadata calculation. No file opening, decoding, tempo changes or truncation.
pub fn plan_audio_timeline(
    request: &AudioTimelineRequest,
) -> Result<AudioTimeline, Vec<TimelineConflict>> {
    let count = request.utterances.len();
    if count == 0 || count > MAX_ASSEMBLY_UTTERANCES {
        return Err(vec![TimelineConflict::InvalidCount {
            actual: count,
            max: MAX_ASSEMBLY_UTTERANCES,
        }]);
    }
    if request.expected_utterance_ids.len() != count
        || request
            .utterances
            .iter()
            .zip(&request.expected_utterance_ids)
            .any(|(actual, expected)| &actual.utterance_id != expected)
    {
        return Err(vec![TimelineConflict::CoverageMismatch]);
    }
    let mut seen = BTreeSet::new();
    for utterance in &request.utterances {
        if !seen.insert(&utterance.utterance_id) {
            return Err(vec![TimelineConflict::DuplicateUtterance {
                utterance_id: utterance.utterance_id.clone(),
            }]);
        }
    }
    let media_frames = if request.mode == TimingMode::SubtitleWindow {
        match request.source_duration_ms {
            Some(duration) if duration > 0 => {
                Some(milliseconds(duration).map_err(|error| vec![error])?)
            }
            _ => return Err(vec![TimelineConflict::MissingMediaDuration]),
        }
    } else {
        None
    };
    let channels = request.utterances[0].channels;
    let mut spans = Vec::with_capacity(count * 2 + 1);
    let mut conflicts = Vec::new();
    let mut cursor = 0_u64;
    let mut previous_window: Option<(&SpeechId, u64)> = None;
    for utterance in &request.utterances {
        let id = &utterance.utterance_id;
        if utterance.frame_count == 0
            || !SUPPORTED_RATES.contains(&utterance.sample_rate)
            || !(1..=2).contains(&utterance.channels)
        {
            conflicts.push(TimelineConflict::InvalidAudio {
                utterance_id: id.clone(),
            });
            continue;
        }
        if utterance.channels != channels {
            conflicts.push(TimelineConflict::ChannelMismatch {
                utterance_id: id.clone(),
                expected: channels,
                actual: utterance.channels,
            });
            continue;
        }
        let frames = match scale_frames(utterance.frame_count, utterance.sample_rate) {
            Ok(value) => value,
            Err(error) => {
                conflicts.push(error);
                continue;
            }
        };
        let start = if request.mode == TimingMode::SubtitleWindow {
            if utterance.pause_after_ms != 0 {
                conflicts.push(TimelineConflict::PauseInSubtitleWindow {
                    utterance_id: id.clone(),
                });
                continue;
            }
            let Some(window) = &utterance.source_window else {
                conflicts.push(TimelineConflict::MissingWindow {
                    utterance_id: id.clone(),
                });
                continue;
            };
            if window.start_ms >= window.end_ms {
                conflicts.push(TimelineConflict::InvalidWindow {
                    utterance_id: id.clone(),
                    start_ms: window.start_ms,
                    end_ms: window.end_ms,
                });
                continue;
            }
            // Presence and positivity were checked before entering the loop.
            let duration = request.source_duration_ms.unwrap_or_default();
            if window.end_ms > duration {
                conflicts.push(TimelineConflict::OutsideMedia {
                    utterance_id: id.clone(),
                    end_ms: window.end_ms,
                    duration_ms: duration,
                });
                continue;
            }
            let overlaps = previous_window.filter(|(_, end)| window.start_ms < *end);
            previous_window = Some((id, window.end_ms));
            if let Some((previous, _)) = overlaps {
                conflicts.push(TimelineConflict::OverlappingWindows {
                    previous_utterance_id: previous.clone(),
                    utterance_id: id.clone(),
                });
                continue;
            }
            let available = match milliseconds(window.end_ms - window.start_ms) {
                Ok(value) => value,
                Err(error) => {
                    conflicts.push(error);
                    continue;
                }
            };
            if frames > available {
                conflicts.push(TimelineConflict::ImpossibleFit {
                    utterance_id: id.clone(),
                    required_frames: frames,
                    available_frames: available,
                });
                continue;
            }
            match milliseconds(window.start_ms) {
                Ok(value) => value,
                Err(error) => {
                    conflicts.push(error);
                    continue;
                }
            }
        } else {
            cursor
        };
        if start > cursor {
            spans.push(TimelineSpan::Silence {
                start_frame: cursor,
                frame_count: start - cursor,
            });
        }
        let Some(end) = start.checked_add(frames) else {
            conflicts.push(TimelineConflict::ArithmeticOverflow);
            continue;
        };
        spans.push(TimelineSpan::Speech {
            utterance_id: id.clone(),
            audio_sha256: utterance.audio_sha256.clone(),
            raw_sample_rate: utterance.sample_rate,
            raw_frame_count: utterance.frame_count,
            source_window: utterance.source_window.clone(),
            start_frame: start,
            frame_count: frames,
        });
        cursor = end;
        if request.mode == TimingMode::Continuous && utterance.pause_after_ms > 0 {
            match milliseconds(utterance.pause_after_ms).and_then(|pause| {
                cursor
                    .checked_add(pause)
                    .map(|end| (pause, end))
                    .ok_or(TimelineConflict::ArithmeticOverflow)
            }) {
                Ok((pause, end)) => {
                    spans.push(TimelineSpan::Silence {
                        start_frame: cursor,
                        frame_count: pause,
                    });
                    cursor = end;
                }
                Err(error) => conflicts.push(error),
            }
        }
    }
    if !conflicts.is_empty() {
        return Err(conflicts);
    }
    if let Some(media_end) = media_frames {
        if media_end > cursor {
            spans.push(TimelineSpan::Silence {
                start_frame: cursor,
                frame_count: media_end - cursor,
            });
        }
        cursor = media_end;
    }
    if u128::from(cursor) * u128::from(channels) * 2 > (1_u128 << 32) - 128 {
        return Err(vec![TimelineConflict::RiffSizeLimit {
            frames: cursor,
            channels,
        }]);
    }
    Ok(AudioTimeline {
        version: 1,
        policy: AUDIO_TIMELINE_POLICY,
        source_plan_sha256: request.source_plan_sha256.clone(),
        raw_manifest_sha256: request.raw_manifest_sha256.clone(),
        mode: request.mode,
        sample_rate: ASSEMBLY_SAMPLE_RATE,
        channels,
        frame_count: cursor,
        spans,
    })
}
