use domain::speech::timeline::*;
use domain::speech::{Sha256Digest, SpeechId, TimingMode};

fn id(value: &str) -> SpeechId {
    match SpeechId::parse(value) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

fn digest(value: u64) -> Sha256Digest {
    match Sha256Digest::parse(format!("{value:064x}")) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    }
}

fn utterance(name: &str, frames: u64) -> MeasuredUtterance {
    MeasuredUtterance {
        utterance_id: id(name),
        audio_sha256: digest(frames),
        sample_rate: 48000,
        channels: 1,
        frame_count: frames,
        source_window: None,
        pause_after_ms: 0,
    }
}

fn request(mode: TimingMode, utterances: Vec<MeasuredUtterance>) -> AudioTimelineRequest {
    AudioTimelineRequest {
        source_plan_sha256: digest(1),
        raw_manifest_sha256: digest(2),
        mode,
        expected_utterance_ids: utterances
            .iter()
            .map(|item| item.utterance_id.clone())
            .collect(),
        utterances,
        source_duration_ms: Some(6000),
    }
}

fn window(item: &mut MeasuredUtterance, start_ms: u64, end_ms: u64) {
    item.source_window = Some(SourceWindow { start_ms, end_ms });
}

fn planned(request: &AudioTimelineRequest) -> AudioTimeline {
    match plan_audio_timeline(request) {
        Ok(value) => value,
        Err(error) => panic!("unexpected conflicts: {error:?}"),
    }
}

fn errors(request: &AudioTimelineRequest) -> Vec<TimelineConflict> {
    match plan_audio_timeline(request) {
        Err(value) => value,
        Ok(value) => panic!("unexpected timeline: {value:?}"),
    }
}

fn geometry(span: &TimelineSpan) -> (u64, u64) {
    match span {
        TimelineSpan::Speech {
            start_frame,
            frame_count,
            ..
        }
        | TimelineSpan::Silence {
            start_frame,
            frame_count,
        } => (*start_frame, *frame_count),
    }
}

#[test]
fn continuous_keeps_600_complete_ordered_utterances_and_only_declared_pauses() {
    let mut facts = (0..600)
        .map(|index| utterance(&format!("cue-{index}"), 480))
        .collect::<Vec<_>>();
    facts[0].pause_after_ms = 250;
    facts[599].pause_after_ms = 100;
    let input = request(TimingMode::Continuous, facts);
    let original = input.clone();
    let timeline = planned(&input);
    assert_eq!(timeline.frame_count, 600 * 480 + 350 * 48);
    assert_eq!(timeline.sample_rate, 48000);
    assert_eq!(timeline.spans.len(), 602);
    let mut cursor = 0;
    let mut speech = 0;
    for span in &timeline.spans {
        let (start, frames) = geometry(span);
        assert_eq!(start, cursor);
        cursor += frames;
        if let TimelineSpan::Speech {
            utterance_id,
            audio_sha256,
            raw_frame_count,
            ..
        } = span
        {
            assert_eq!(utterance_id, &input.utterances[speech].utterance_id);
            assert_eq!(audio_sha256, &input.utterances[speech].audio_sha256);
            assert_eq!(*raw_frame_count, 480);
            speech += 1;
        }
    }
    assert_eq!(cursor, timeline.frame_count);
    assert_eq!(speech, 600);
    assert_eq!(input, original);
    assert_eq!(planned(&input), timeline);
}

#[test]
fn resampling_counts_full_fractional_tails_and_preserves_manifest_lineage() {
    let mut first = utterance("first", 7);
    first.sample_rate = 44100;
    let mut second = utterance("second", 1);
    second.sample_rate = 11025;
    let input = request(TimingMode::Continuous, vec![first, second]);
    let timeline = planned(&input);
    assert_eq!(timeline.frame_count, 8 + 5);
    assert_eq!(timeline.source_plan_sha256, input.source_plan_sha256);
    assert_eq!(timeline.raw_manifest_sha256, input.raw_manifest_sha256);
    assert_eq!(timeline.policy, AUDIO_TIMELINE_POLICY);
    let mut changed = input.clone();
    changed.raw_manifest_sha256 = digest(99);
    assert_ne!(planned(&changed), timeline);
}

#[test]
fn source_windows_keep_starts_with_leading_interior_and_trailing_silence() {
    let mut first = utterance("first", 48000);
    window(&mut first, 1000, 2000);
    let mut second = utterance("second", 24000);
    window(&mut second, 3000, 5000);
    let timeline = planned(&request(TimingMode::SubtitleWindow, vec![first, second]));
    let positions = timeline.spans.iter().map(geometry).collect::<Vec<_>>();
    assert_eq!(
        positions,
        vec![
            (0, 48000),
            (48000, 48000),
            (96000, 48000),
            (144000, 24000),
            (168000, 120000)
        ]
    );
    assert_eq!(timeline.frame_count, 288000);
    assert!(matches!(timeline.spans[0], TimelineSpan::Silence { .. }));
    assert!(matches!(timeline.spans[4], TimelineSpan::Silence { .. }));
}

#[test]
fn exact_fit_and_adjacent_windows_need_no_padding_or_tempo_change() {
    let mut first = utterance("first", 24000);
    first.sample_rate = 24000;
    window(&mut first, 0, 1000);
    let mut second = utterance("second", 48);
    window(&mut second, 1000, 1001);
    let mut input = request(TimingMode::SubtitleWindow, vec![first, second]);
    input.source_duration_ms = Some(1001);
    let timeline = planned(&input);
    assert_eq!(timeline.frame_count, 48048);
    assert_eq!(timeline.spans.len(), 2);
    assert!(
        timeline
            .spans
            .iter()
            .all(|span| matches!(span, TimelineSpan::Speech { .. }))
    );
}

#[test]
fn a_single_excess_frame_is_an_actionable_impossible_fit() {
    let mut item = utterance("too-long", 48001);
    window(&mut item, 0, 1000);
    let input = request(TimingMode::SubtitleWindow, vec![item]);
    assert_eq!(
        errors(&input),
        vec![TimelineConflict::ImpossibleFit {
            utterance_id: id("too-long"),
            required_frames: 48001,
            available_frames: 48000,
        }]
    );
}

#[test]
fn overlapping_source_windows_conflict_even_when_the_speech_itself_is_short() {
    let mut first = utterance("first", 48);
    window(&mut first, 0, 2000);
    let mut second = utterance("second", 48);
    window(&mut second, 1000, 3000);
    let input = request(TimingMode::SubtitleWindow, vec![first, second]);
    assert_eq!(
        errors(&input),
        vec![TimelineConflict::OverlappingWindows {
            previous_utterance_id: id("first"),
            utterance_id: id("second"),
        }]
    );
}

#[test]
fn missing_invalid_outside_windows_and_subtitle_pauses_are_refused() {
    let baseline = request(TimingMode::SubtitleWindow, vec![utterance("one", 48)]);
    assert!(matches!(
        errors(&baseline)[0],
        TimelineConflict::MissingWindow { .. }
    ));
    for (start, end) in [(1000, 999), (1000, 1000)] {
        let mut input = baseline.clone();
        window(&mut input.utterances[0], start, end);
        assert!(matches!(
            errors(&input)[0],
            TimelineConflict::InvalidWindow { .. }
        ));
    }
    let mut input = baseline.clone();
    window(&mut input.utterances[0], 1000, 6001);
    assert!(matches!(
        errors(&input)[0],
        TimelineConflict::OutsideMedia { .. }
    ));
    window(&mut input.utterances[0], 0, 1000);
    input.utterances[0].pause_after_ms = 1;
    assert!(matches!(
        errors(&input)[0],
        TimelineConflict::PauseInSubtitleWindow { .. }
    ));
    for duration in [None, Some(0)] {
        input.source_duration_ms = duration;
        assert_eq!(errors(&input), vec![TimelineConflict::MissingMediaDuration]);
    }
}

#[test]
fn missing_extra_reordered_and_duplicate_coverage_never_yield_a_timeline() {
    let baseline = request(
        TimingMode::Continuous,
        vec![utterance("one", 48), utterance("two", 48)],
    );
    let mut input = baseline.clone();
    input.utterances.pop();
    assert_eq!(errors(&input), vec![TimelineConflict::CoverageMismatch]);
    let mut input = baseline.clone();
    input.utterances.push(utterance("extra", 48));
    assert_eq!(errors(&input), vec![TimelineConflict::CoverageMismatch]);
    let mut input = baseline;
    input.utterances.reverse();
    assert_eq!(errors(&input), vec![TimelineConflict::CoverageMismatch]);
    let input = request(TimingMode::Continuous, vec![utterance("duplicate", 48); 2]);
    assert_eq!(
        errors(&input),
        vec![TimelineConflict::DuplicateUtterance {
            utterance_id: id("duplicate")
        }]
    );
}

#[test]
fn invalid_raw_properties_channel_mismatch_and_both_overflow_classes_fail() {
    for (rate, channels, frames) in [
        (0, 1, 48),
        (12345, 1, 48),
        (48000, 0, 48),
        (48000, 3, 48),
        (48000, 1, 0),
    ] {
        let mut item = utterance("invalid", frames);
        item.sample_rate = rate;
        item.channels = channels;
        assert!(matches!(
            errors(&request(TimingMode::Continuous, vec![item]))[0],
            TimelineConflict::InvalidAudio { .. }
        ));
    }
    let mut stereo = utterance("stereo", 48);
    stereo.channels = 2;
    assert!(matches!(
        errors(&request(
            TimingMode::Continuous,
            vec![utterance("mono", 48), stereo]
        ))[0],
        TimelineConflict::ChannelMismatch {
            expected: 1,
            actual: 2,
            ..
        }
    ));
    let item = utterance("too-large", 1_u64 << 32);
    assert!(matches!(
        errors(&request(TimingMode::Continuous, vec![item]))[0],
        TimelineConflict::RiffSizeLimit { .. }
    ));
    let mut item = utterance("overflow", u64::MAX);
    item.sample_rate = 8000;
    assert_eq!(
        errors(&request(TimingMode::Continuous, vec![item])),
        vec![TimelineConflict::ArithmeticOverflow]
    );
    let mut item = utterance("pause-overflow", 48);
    item.pause_after_ms = u64::MAX;
    assert_eq!(
        errors(&request(TimingMode::Continuous, vec![item])),
        vec![TimelineConflict::ArithmeticOverflow]
    );
    let mut input = request(TimingMode::SubtitleWindow, vec![utterance("one", 48)]);
    input.source_duration_ms = Some(u64::MAX);
    assert_eq!(errors(&input), vec![TimelineConflict::ArithmeticOverflow]);
}

#[test]
fn empty_over_budget_and_exact_maximum_metadata_are_bounded() {
    let input = request(TimingMode::Continuous, vec![]);
    assert_eq!(
        errors(&input),
        vec![TimelineConflict::InvalidCount {
            actual: 0,
            max: 8192
        }]
    );
    let input = request(TimingMode::Continuous, vec![utterance("one", 1); 8193]);
    assert_eq!(
        errors(&input),
        vec![TimelineConflict::InvalidCount {
            actual: 8193,
            max: 8192
        }]
    );
    let facts = (0..8192)
        .map(|index| utterance(&format!("u-{index}"), 1))
        .collect();
    let timeline = planned(&request(TimingMode::Continuous, facts));
    assert_eq!(timeline.frame_count, 8192);
    assert_eq!(timeline.spans.len(), 8192);
}
