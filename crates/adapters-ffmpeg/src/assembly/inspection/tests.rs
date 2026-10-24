use super::super::{AudioAssemblyLimits, pcm};
use super::*;
use domain::speech::{
    Sha256Digest, SpeechId, TimingMode,
    timeline::{SourceWindow, TimelineConflict},
};
use ports::{
    PortError,
    audio_inspection::{
        ManagedAudioInput, SpeechAudioInspectionPort, SpeechAudioInspectionRequest,
        SpeechTimingSettings,
    },
    cancellation::CancellationToken,
    speech_audio::{SpeechAudioLayout, UtteranceAudioRange},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};
use tempfile::TempDir;

struct Fixture {
    root: TempDir,
    request: SpeechAudioInspectionRequest,
    layout: SpeechAudioLayout,
    manifest: Value,
    pcm: Vec<u8>,
}

fn sha(bytes: &[u8]) -> Result<Sha256Digest, &'static str> {
    Sha256Digest::parse(format!("{:x}", Sha256::digest(bytes)))
}

fn input(path: &Path) -> Result<ManagedAudioInput, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(ManagedAudioInput {
        path: path.into(),
        size_bytes: bytes.len() as u64,
        sha256: sha(&bytes)?,
    })
}

impl Fixture {
    fn new(counts: &[u64]) -> Result<Self, Box<dyn std::error::Error>> {
        let parent =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/revoice/inspection-unit");
        fs::create_dir_all(&parent)?;
        let root = tempfile::tempdir_in(parent)?;
        let frames: u64 = counts.iter().sum();
        let mut waveform = Vec::new();
        pcm::write_header(&mut waveform, frames * 4, 2)?;
        let mut data = Vec::new();
        for index in 0..frames * 2 {
            data.extend_from_slice(&((index % 30000) as i16 - 15000).to_le_bytes());
        }
        waveform.extend_from_slice(&data);
        let aggregate_path = root.path().join("管理's.wav");
        fs::write(&aggregate_path, waveform)?;
        let aggregate = input(&aggregate_path)?;
        let mut offset = 0;
        let mut ranges = Vec::new();
        let mut segments = Vec::new();
        for (index, count) in counts.iter().enumerate() {
            let id = SpeechId::parse(format!("line-{index}"))?;
            let chunks = vec![
                format!("line-{index}-chunk-0001"),
                format!("line-{index}-chunk-0002"),
            ];
            for chunk in &chunks {
                segments.push(json!({"utterance_id":id, "chunk_id":chunk, "kind":"text", "status":"cached", "audio":"E:/unavailable/cache.wav"}));
            }
            ranges.push(UtteranceAudioRange {
                utterance_id: id,
                first_frame: offset,
                frame_count: *count,
                chunk_ids: chunks,
                source_window: Some(SourceWindow {
                    start_ms: index as u64 * 10 + 2,
                    end_ms: index as u64 * 10 + 7,
                }),
            });
            offset += count;
        }
        let manifest = json!({"version":4,"audio_assembly":"pcm16_stream_fade10ms_peak089125_kaiser5_v1", "sample_rate":48000,"output":"E:/unavailable/attempt/raw.wav", "output_sha256":aggregate.sha256,"speech_plan":{"expected_utterance_ids":ranges.iter().map(|item| item.utterance_id.clone()).collect::<Vec<_>>()},"segments":segments});
        let manifest_path = root.path().join("raw-manifest.json");
        fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
        let manifest_input = input(&manifest_path)?;
        let layout = SpeechAudioLayout {
            aggregate_path: root.path().join("gone/attempt/raw.wav"),
            aggregate_sha256: aggregate.sha256.clone(),
            source_plan_sha256: Sha256Digest::parse("a".repeat(64))?,
            raw_manifest_sha256: manifest_input.sha256.clone(),
            assembly_policy: "pcm16_stream_fade10ms_peak089125_kaiser5_v1".into(),
            sample_rate: 48000,
            channels: 2,
            frame_count: frames,
            utterances: ranges,
        };
        let layout_path = root.path().join("audio-layout.json");
        fs::write(&layout_path, serde_json::to_vec(&layout)?)?;
        let request = SpeechAudioInspectionRequest {
            aggregate,
            layout: input(&layout_path)?,
            manifest: manifest_input,
            source_plan_sha256: layout.source_plan_sha256.clone(),
            expected_utterance_ids: layout
                .utterances
                .iter()
                .map(|item| item.utterance_id.clone())
                .collect(),
            settings: SpeechTimingSettings {
                mode: TimingMode::Continuous,
                source_duration_ms: None,
                pause_after_ms: vec![0; counts.len()],
            },
        };
        Ok(Self {
            root,
            request,
            layout,
            manifest,
            pcm: data,
        })
    }

    fn reader(&self) -> Result<ManagedSpeechAudioReader, PortError> {
        ManagedSpeechAudioReader::new(self.root.path().into(), AudioAssemblyLimits::default())
    }

    fn save_metadata(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.request.aggregate = input(&self.request.aggregate.path)?;
        self.layout.aggregate_sha256 = self.request.aggregate.sha256.clone();
        self.manifest["output_sha256"] = json!(self.request.aggregate.sha256);
        fs::write(
            &self.request.manifest.path,
            serde_json::to_vec(&self.manifest)?,
        )?;
        self.request.manifest = input(&self.request.manifest.path)?;
        self.layout.raw_manifest_sha256 = self.request.manifest.sha256.clone();
        fs::write(&self.request.layout.path, serde_json::to_vec(&self.layout)?)?;
        self.request.layout = input(&self.request.layout.path)?;
        Ok(())
    }

    fn expected_wave(&self, offset: u64, frames: u64) -> Vec<u8> {
        let size = frames as u32 * 4;
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&(size + 36).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&48000_u32.to_le_bytes());
        bytes.extend_from_slice(&192000_u32.to_le_bytes());
        bytes.extend_from_slice(&4_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&size.to_le_bytes());
        bytes.extend_from_slice(&self.pcm[(offset * 4) as usize..((offset + frames) * 4) as usize]);
        bytes
    }
}

#[tokio::test]
async fn rebinds_a_vanished_attempt_to_ready_audio_and_hashes_complete_utterance_waves()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[184, 50])?;
    assert!(!fixture.layout.aggregate_path.exists());
    let result = fixture
        .reader()?
        .inspect(fixture.request.clone(), CancellationToken::new())
        .await?;
    assert!(result.conflicts.is_empty());
    assert_eq!(
        result.source.aggregate_path,
        fs::canonicalize(&fixture.request.aggregate.path)?
    );
    assert_eq!(
        result
            .timeline
            .as_ref()
            .map(|timeline| timeline.frame_count),
        Some(234)
    );
    assert_eq!(
        result.timeline_request.utterances[0].audio_sha256,
        sha(&fixture.expected_wave(0, 184))?
    );
    assert_eq!(
        result.timeline_request.utterances[1].audio_sha256,
        sha(&fixture.expected_wave(184, 50))?
    );
    assert_eq!(fs::read_dir(fixture.root.path())?.count(), 3);
    Ok(())
}

#[tokio::test]
async fn extra_riff_metadata_is_not_part_of_a_canonical_utterance_wave()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    let mut bytes = fs::read(&fixture.request.aggregate.path)?;
    bytes.splice(36..36, *b"JUNK\x04\0\0\0test");
    let riff_len = bytes.len() as u32 - 8;
    bytes[4..8].copy_from_slice(&riff_len.to_le_bytes());
    fs::write(&fixture.request.aggregate.path, bytes)?;
    fixture.save_metadata()?;
    let result = fixture
        .reader()?
        .inspect(fixture.request.clone(), CancellationToken::new())
        .await?;
    assert_eq!(
        result.timeline_request.utterances[0].audio_sha256,
        sha(&fixture.expected_wave(0, 184))?
    );
    Ok(())
}

#[tokio::test]
async fn reports_typed_impossible_fit_and_valid_window_silence()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    fixture.request.settings.mode = TimingMode::SubtitleWindow;
    fixture.request.settings.source_duration_ms = Some(20);
    let valid = fixture
        .reader()?
        .inspect(fixture.request.clone(), CancellationToken::new())
        .await?;
    assert_eq!(
        valid.timeline.map(|timeline| timeline.frame_count),
        Some(960)
    );
    fixture.layout.utterances[0].source_window = Some(SourceWindow {
        start_ms: 2,
        end_ms: 3,
    });
    fixture.save_metadata()?;
    let result = fixture
        .reader()?
        .inspect(fixture.request.clone(), CancellationToken::new())
        .await?;
    assert!(result.timeline.is_none());
    assert!(result.conflicts.iter().any(|conflict| matches!(
        conflict,
        TimelineConflict::ImpossibleFit {
            required_frames: 184,
            available_frames: 48,
            ..
        }
    )));
    Ok(())
}

#[tokio::test]
async fn corrupt_wave_or_layout_hash_never_yields_accepted_measurements()
-> Result<(), Box<dyn std::error::Error>> {
    for layout_corrupt in [false, true] {
        let fixture = Fixture::new(&[184, 50])?;
        let path = if layout_corrupt {
            &fixture.request.layout.path
        } else {
            &fixture.request.aggregate.path
        };
        let mut bytes = fs::read(path)?;
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(path, bytes)?;
        assert!(
            fixture
                .reader()?
                .inspect(fixture.request.clone(), CancellationToken::new())
                .await
                .is_err()
        );
    }
    Ok(())
}

#[tokio::test]
async fn rejects_wrong_selected_plan_or_order_and_unknown_policy()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    let mut changed = fixture.request.clone();
    changed.source_plan_sha256 = Sha256Digest::parse("b".repeat(64))?;
    assert!(
        fixture
            .reader()?
            .inspect(changed, CancellationToken::new())
            .await
            .is_err()
    );
    changed = fixture.request.clone();
    changed.expected_utterance_ids.reverse();
    assert!(
        fixture
            .reader()?
            .inspect(changed, CancellationToken::new())
            .await
            .is_err()
    );
    fixture.layout.assembly_policy = "unknown".into();
    fixture.save_metadata()?;
    assert!(
        fixture
            .reader()?
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn rejects_reordered_manifest_chunks_even_with_matching_retained_hashes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    fixture.manifest["segments"]
        .as_array_mut()
        .ok_or("Missing segments")?
        .swap(0, 1);
    fixture.save_metadata()?;
    assert!(
        fixture
            .reader()?
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn rejects_oversized_json_before_allocating_and_pcm_over_input_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    let reader = ManagedSpeechAudioReader::new(
        fixture.root.path().into(),
        AudioAssemblyLimits {
            max_input_bytes: fixture.request.aggregate.size_bytes - 1,
            ..AudioAssemblyLimits::default()
        },
    )?;
    assert!(
        reader
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&fixture.request.layout.path)?;
    file.set_len(files::MAX_JSON_BYTES + 1)?;
    fixture.request.layout.size_bytes = files::MAX_JSON_BYTES + 1;
    assert!(
        fixture
            .reader()?
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn outside_managed_root_is_refused_without_modification()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new(&[184, 50])?;
    let outside = tempfile::tempdir_in(fixture.root.path().parent().ok_or("Missing parent")?)?;
    let path = outside.path().join("outside.wav");
    fs::copy(&fixture.request.aggregate.path, &path)?;
    let before = fs::read(&path)?;
    fixture.request.aggregate.path = path.clone();
    assert!(
        fixture
            .reader()?
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    assert_eq!(fs::read(path)?, before);
    Ok(())
}

#[tokio::test]
async fn cancellation_and_expired_deadline_are_not_success()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[184, 50])?;
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        fixture
            .reader()?
            .inspect(fixture.request.clone(), cancel)
            .await,
        Err(PortError::Cancelled)
    ));
    let reader = ManagedSpeechAudioReader::new(
        fixture.root.path().into(),
        AudioAssemblyLimits {
            process_timeout: Duration::from_nanos(1),
            total_timeout: Duration::from_nanos(1),
            ..AudioAssemblyLimits::default()
        },
    )?;
    assert!(
        reader
            .inspect(fixture.request.clone(), CancellationToken::new())
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn inspects_six_hundred_utterances_with_no_cache_or_runtime_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&vec![120; 600])?;
    let result = fixture
        .reader()?
        .inspect(fixture.request.clone(), CancellationToken::new())
        .await?;
    assert_eq!(result.timeline_request.utterances.len(), 600);
    assert_eq!(
        result
            .timeline
            .as_ref()
            .map(|timeline| timeline.frame_count),
        Some(72000)
    );
    assert_eq!(
        result.timeline_request.utterances[599].audio_sha256,
        sha(&fixture.expected_wave(71880, 120))?
    );
    assert_eq!(fs::read_dir(fixture.root.path())?.count(), 3);
    Ok(())
}

#[tokio::test]
async fn queued_inspection_is_cancellable_and_a_later_request_can_proceed()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[120])?;
    let reader = fixture.reader()?;
    let permit = reader.admission_for_test().acquire_owned().await?;
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let queued_reader = reader.clone();
    let request = fixture.request.clone();
    let task = tokio::spawn(async move { queued_reader.inspect(request, token).await });
    tokio::task::yield_now().await;
    cancel.cancel();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), task).await??,
        Err(PortError::Cancelled)
    ));
    drop(permit);
    assert!(
        reader
            .inspect(fixture.request, CancellationToken::new())
            .await?
            .timeline
            .is_some()
    );
    Ok(())
}
