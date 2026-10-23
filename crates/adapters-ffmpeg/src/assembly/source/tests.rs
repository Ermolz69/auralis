use super::*;
use crate::assembly::{control::WorkControl, pcm::PcmStats};
use domain::speech::{SpeechId, TimingMode, timeline::SourceWindow};
use ports::{cancellation::CancellationToken, speech_audio::UtteranceAudioRange};
use sha2::{Digest, Sha256};
use std::{io::Write, time::Duration};
use tempfile::TempDir;

struct Fixture {
    directory: TempDir,
    request: SpeechAudioAssemblyRequest,
    original: Vec<u8>,
}

impl Fixture {
    fn new(count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/bridge-unit");
        fs::create_dir_all(&parent)?;
        let directory = tempfile::tempdir_in(parent)?;
        fs::create_dir(directory.path().join("staging"))?;
        let path = directory.path().join("原始's.wav");
        let frames = 120 * count as u64;
        let mut file = File::create(&path)?;
        pcm::write_header(&mut file, frames * 4, 2)?;
        for index in 0..frames * 2 {
            file.write_all(&((index % 32768) as i16 - 16000).to_le_bytes())?;
        }
        file.sync_all()?;
        drop(file);
        let original = fs::read(&path)?;
        let hash = Sha256Digest::parse(format!("{:x}", Sha256::digest(&original)))?;
        let mut ranges = Vec::new();
        for index in 0..count {
            ranges.push(UtteranceAudioRange {
                utterance_id: SpeechId::parse(format!("line-{index}"))?,
                first_frame: index as u64 * 120,
                frame_count: 120,
                chunk_ids: vec![
                    format!("line-{index}-chunk-0001"),
                    format!("line-{index}-chunk-0002"),
                ],
                source_window: Some(SourceWindow {
                    start_ms: index as u64 * 10,
                    end_ms: index as u64 * 10 + 5,
                }),
            });
        }
        Ok(Self {
            directory,
            original,
            request: SpeechAudioAssemblyRequest {
                source: SpeechAudioLayout {
                    aggregate_path: path,
                    aggregate_sha256: hash.clone(),
                    source_plan_sha256: hash.clone(),
                    raw_manifest_sha256: hash,
                    assembly_policy: POLICY.into(),
                    sample_rate: 48000,
                    channels: 2,
                    frame_count: frames,
                    utterances: ranges,
                },
                mode: TimingMode::Continuous,
                source_duration_ms: None,
                pause_after_ms: vec![0; count],
            },
        })
    }

    fn run(
        &self,
        request: SpeechAudioAssemblyRequest,
        limits: &AudioAssemblyLimits,
        cancel: CancellationToken,
    ) -> Result<PreparedAssembly, PortError> {
        prepare(
            request,
            &fs::canonicalize(self.directory.path()).map_err(error::io)?,
            &self.directory.path().join("staging"),
            limits,
            &WorkControl::new(cancel, Duration::from_secs(20))?,
        )
    }

    fn staging_is_empty(&self) -> Result<bool, std::io::Error> {
        Ok(fs::read_dir(self.directory.path().join("staging"))?
            .next()
            .is_none())
    }
}

#[test]
fn extracts_complete_stereo_utterances_without_changing_any_sample()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    let prepared = fixture.run(
        fixture.request.clone(),
        &AudioAssemblyLimits::default(),
        CancellationToken::new(),
    )?;
    let first = fs::read(prepared.directory.path().join("speech-00000.wav"))?;
    let second = fs::read(prepared.directory.path().join("speech-00001.wav"))?;
    assert_eq!(&first[44..], &fixture.original[44..44 + 480]);
    assert_eq!(&second[44..], &fixture.original[44 + 480..]);
    assert_eq!(
        prepared.pcm_sha256,
        format!("{:x}", Sha256::digest(&fixture.original[44..]))
    );
    assert_eq!(prepared.timeline.frame_count, 240);
    assert_eq!(prepared.raw_source, Some(fixture.request.source.clone()));
    assert_eq!(
        fs::read(&fixture.request.source.aggregate_path)?,
        fixture.original
    );
    let directories: Vec<_> = fs::read_dir(fixture.directory.path().join("staging"))?.collect();
    assert_eq!(directories.len(), 1);
    Ok(())
}

#[test]
fn stale_aggregate_hash_never_leaves_partial_staging() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    let mut bytes = fixture.original.clone();
    bytes[44] ^= 1;
    fs::write(&fixture.request.source.aggregate_path, bytes)?;
    assert!(
        fixture
            .run(
                fixture.request.clone(),
                &AudioAssemblyLimits::default(),
                CancellationToken::new()
            )
            .is_err()
    );
    assert!(fixture.staging_is_empty()?);
    Ok(())
}

#[test]
fn unknown_policy_and_stale_pcm_properties_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    for variant in 0..4 {
        let mut request = fixture.request.clone();
        match variant {
            0 => request.source.assembly_policy = "unknown".into(),
            1 => request.source.channels = 1,
            2 => request.source.sample_rate = 24000,
            _ => request.pause_after_ms.clear(),
        }
        assert!(
            fixture
                .run(
                    request,
                    &AudioAssemblyLimits::default(),
                    CancellationToken::new()
                )
                .is_err()
        );
        assert!(fixture.staging_is_empty()?);
    }
    Ok(())
}

#[test]
fn rejects_gaps_overlaps_duplicates_and_incomplete_chunk_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    for variant in 0..6 {
        let mut request = fixture.request.clone();
        match variant {
            0 => request.source.utterances[1].first_frame += 1,
            1 => request.source.utterances[1].first_frame -= 1,
            2 => {
                request.source.utterances[1].utterance_id =
                    request.source.utterances[0].utterance_id.clone()
            }
            3 => request.source.utterances[1].frame_count -= 1,
            4 => request.source.utterances[0].chunk_ids.reverse(),
            _ => request.source.utterances[1].chunk_ids.clear(),
        }
        assert!(
            fixture
                .run(
                    request,
                    &AudioAssemblyLimits::default(),
                    CancellationToken::new()
                )
                .is_err()
        );
        assert!(fixture.staging_is_empty()?);
    }
    Ok(())
}

#[test]
fn budget_counts_snapshot_and_every_extracted_wav_header() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = Fixture::new(2)?;
    let exact = fixture.original.len() as u64 + 2 * (480 + 44);
    let mut limits = AudioAssemblyLimits {
        max_input_bytes: exact - 1,
        ..AudioAssemblyLimits::default()
    };
    assert!(
        fixture
            .run(fixture.request.clone(), &limits, CancellationToken::new())
            .is_err()
    );
    assert!(fixture.staging_is_empty()?);
    limits.max_input_bytes = exact;
    assert!(
        fixture
            .run(fixture.request.clone(), &limits, CancellationToken::new())
            .is_ok()
    );
    Ok(())
}

#[test]
fn explicit_window_conflicts_remain_failures_after_extraction()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    let mut request = fixture.request.clone();
    request.mode = TimingMode::SubtitleWindow;
    request.source_duration_ms = Some(20);
    request.source.utterances[0].source_window = Some(SourceWindow {
        start_ms: 0,
        end_ms: 1,
    });
    assert!(
        fixture
            .run(
                request,
                &AudioAssemblyLimits::default(),
                CancellationToken::new()
            )
            .is_err()
    );
    assert!(fixture.staging_is_empty()?);
    Ok(())
}

#[test]
fn source_windows_keep_speech_and_insert_exact_leading_and_trailing_silence()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    let mut request = fixture.request.clone();
    request.mode = TimingMode::SubtitleWindow;
    request.source_duration_ms = Some(20);
    request.source.utterances[0].source_window = Some(SourceWindow {
        start_ms: 2,
        end_ms: 7,
    });
    let prepared = fixture.run(
        request,
        &AudioAssemblyLimits::default(),
        CancellationToken::new(),
    )?;
    let mut expected = vec![0_u8; 96 * 4];
    expected.extend_from_slice(&fixture.original[44..44 + 480]);
    expected.resize(480 * 4, 0);
    expected.extend_from_slice(&fixture.original[44 + 480..]);
    expected.resize(960 * 4, 0);
    assert_eq!(prepared.timeline.frame_count, 960);
    assert_eq!(
        prepared.pcm_sha256,
        format!("{:x}", Sha256::digest(&expected))
    );
    assert_eq!(prepared.stats.samples, 1920);
    Ok(())
}

#[test]
fn cancellation_or_expired_deadline_creates_no_staging() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(2)?;
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        fixture.run(
            fixture.request.clone(),
            &AudioAssemblyLimits::default(),
            cancel
        ),
        Err(PortError::Cancelled)
    ));
    let control = WorkControl {
        cancel: CancellationToken::new(),
        deadline: std::time::Instant::now() - Duration::from_secs(1),
    };
    assert!(
        prepare(
            fixture.request.clone(),
            fixture.directory.path(),
            &fixture.directory.path().join("staging"),
            &AudioAssemblyLimits::default(),
            &control
        )
        .is_err()
    );
    assert!(fixture.staging_is_empty()?);
    Ok(())
}

#[test]
fn six_hundred_ranges_preserve_every_stereo_sample_and_order()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(600)?;
    let prepared = fixture.run(
        fixture.request.clone(),
        &AudioAssemblyLimits::default(),
        CancellationToken::new(),
    )?;
    assert_eq!(
        prepared.pcm_sha256,
        format!("{:x}", Sha256::digest(&fixture.original[44..]))
    );
    assert_eq!(prepared.timeline.frame_count, 72000);
    assert_eq!(prepared.stats.samples, 144000);
    assert_eq!(
        prepared
            .raw_source
            .as_ref()
            .map(|value| value.utterances.len()),
        Some(600)
    );
    let mut digest = Sha256::new();
    let mut stats = PcmStats::default();
    let last = pcm::append_pcm(
        &prepared.directory.path().join("speech-00599.wav"),
        &mut digest,
        &mut stats,
        &WorkControl::new(CancellationToken::new(), Duration::from_secs(10))?,
    )?;
    assert_eq!(last.frames, 120);
    assert_eq!(
        format!("{:x}", digest.finalize()),
        format!("{:x}", Sha256::digest(&fixture.original[44 + 599 * 480..]))
    );
    Ok(())
}
