use super::*;
use adapters_tts::verify_result_audio_layout;

fn wav(rate: u32, frames: u64) -> Vec<u8> {
    let mut bytes = minimal_wav();
    bytes[4..8].copy_from_slice(&(36 + frames as u32 * 2).to_le_bytes());
    bytes[24..28].copy_from_slice(&rate.to_le_bytes());
    bytes[28..32].copy_from_slice(&(rate * 2).to_le_bytes());
    bytes[40..44].copy_from_slice(&(frames as u32 * 2).to_le_bytes());
    bytes.resize(44 + frames as usize * 2, 0);
    bytes
}

fn fixture(groups: &[Vec<(u32, u64)>]) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new()?;
    let mut plan: Value = serde_json::from_slice(&fs::read(&fixture.request.plan_path)?)?;
    let utterance = plan["utterances"][0].clone();
    let segment = fixture.manifest["segments"][0].clone();
    let mut utterances = Vec::new();
    let mut segments = Vec::new();
    let mut expected_ids = Vec::new();
    let mut total_frames = 0_u64;
    for (index, group) in groups.iter().enumerate() {
        let id = format!("line-{index}");
        let mut item = utterance.clone();
        item["utterance_id"] = json!(id);
        item["text"] = json!("part ".repeat(group.len()));
        for (chunk_index, (rate, frames)) in group.iter().enumerate() {
            let path = fixture
                .request
                .cache_dir
                .join(format!("{index}-{chunk_index}.wav"));
            fs::write(&path, wav(*rate, *frames))?;
            let mut chunk = segment.clone();
            chunk["utterance_id"] = json!(id);
            chunk["chunk_id"] = json!(format!("{id}-chunk-{:04}", chunk_index + 1));
            chunk["text"] = json!("part ");
            chunk["seed_key"] = json!(format!("seed-1:chunk:{}", chunk_index + 1));
            chunk["audio"] = json!(path.to_string_lossy());
            chunk["audio_sha256"] = json!(digest(&path)?);
            segments.push(chunk);
            total_frames += (frames * 48000).div_ceil(u64::from(*rate));
        }
        utterances.push(item);
        expected_ids.push(id);
    }
    plan["utterances"] = json!(utterances);
    fs::write(&fixture.request.plan_path, serde_json::to_vec(&plan)?)?;
    fs::write(&fixture.request.out_file, wav(48000, total_frames))?;
    fixture.completed.frames = total_frames;
    fixture.completed.duration_ms =
        (total_frames as f64 * 1000.0 / 48000.0).round_ties_even() as u64;
    fixture.completed.utterance_count = groups.len();
    fixture.completed.chunk_count = segments.len();
    fixture.completed.output_sha256 = digest(&fixture.request.out_file)?;
    fixture.request.expected_chunks = segments.len();
    fixture.request.expected_utterances = groups.len();
    fixture.manifest["audio_assembly"] = json!("pcm16_stream_fade10ms_peak089125_kaiser5_v1");
    fixture.manifest["output_sha256"] = json!(fixture.completed.output_sha256);
    fixture.manifest["segments"] = json!(segments);
    fixture.manifest["speech_plan"]["expected_utterance_ids"] = json!(expected_ids);
    fixture.save_manifest()?;
    Ok(fixture)
}

#[test]
fn whole_utterance_ranges_use_per_chunk_ceiling_and_exact_lineage()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(&[vec![(44100, 101), (24000, 37)], vec![(48000, 50)]])?;
    let layout = verify_result_audio_layout(&fixture.request, &fixture.completed)?;
    assert_eq!(layout.frame_count, 234);
    assert_eq!(layout.utterances[0].first_frame, 0);
    assert_eq!(layout.utterances[0].frame_count, 184);
    assert_eq!(
        layout.utterances[0].chunk_ids,
        ["line-0-chunk-0001", "line-0-chunk-0002"]
    );
    assert_eq!(layout.utterances[1].first_frame, 184);
    assert_eq!(layout.utterances[1].frame_count, 50);
    assert_eq!(
        layout.utterances[0]
            .source_window
            .as_ref()
            .map(|window| (window.start_ms, window.end_ms)),
        Some((100, 2000))
    );
    assert_eq!(
        layout.source_plan_sha256.as_str(),
        digest(&fixture.request.plan_path)?
    );
    assert_eq!(
        layout.raw_manifest_sha256.as_str(),
        fixture.completed.manifest_sha256
    );
    assert_eq!(
        layout.aggregate_sha256.as_str(),
        digest(&fixture.request.out_file)?
    );
    Ok(())
}

#[test]
fn legacy_result_remains_verifiable_but_unknown_layout_policy_is_refused()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = Fixture::new()?;
    verify_result_files(&fixture.request, &fixture.completed)?;
    assert!(verify_result_audio_layout(&fixture.request, &fixture.completed).is_err());
    fixture.manifest["audio_assembly"] = json!("unknown-fade-policy");
    fixture.save_manifest()?;
    assert!(verify_result_audio_layout(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn rejects_extra_aggregate_frame_with_self_consistent_worker_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture(&[vec![(44100, 101), (24000, 37)]])?;
    fixture.completed.frames += 1;
    fs::write(
        &fixture.request.out_file,
        wav(48000, fixture.completed.frames),
    )?;
    fixture.completed.output_sha256 = digest(&fixture.request.out_file)?;
    fixture.manifest["output_sha256"] = json!(fixture.completed.output_sha256);
    fixture.save_manifest()?;
    assert!(verify_result_audio_layout(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn rejects_invalid_chunk_wav_even_when_all_hashes_match() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = fixture(&[vec![(48000, 100)]])?;
    let path = fixture.request.cache_dir.join("0-0.wav");
    fs::write(&path, b"invalid but correctly hashed")?;
    fixture.manifest["segments"][0]["audio_sha256"] = json!(digest(&path)?);
    fixture.save_manifest()?;
    assert!(verify_result_audio_layout(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn rejects_unrecognized_chunk_rate() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(&[vec![(12345, 100)]])?;
    assert!(verify_result_audio_layout(&fixture.request, &fixture.completed).is_err());
    Ok(())
}

#[test]
fn accepts_all_thirteen_streaming_rates() -> Result<(), Box<dyn std::error::Error>> {
    let rates = [
        8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
    ];
    let fixture = fixture(&[rates.iter().map(|rate| (*rate, 1001)).collect()])?;
    let layout = verify_result_audio_layout(&fixture.request, &fixture.completed)?;
    let expected: u64 = rates
        .iter()
        .map(|rate| (1001_u64 * 48000).div_ceil(u64::from(*rate)))
        .sum();
    assert_eq!(layout.utterances[0].frame_count, expected);
    assert_eq!(layout.utterances[0].chunk_ids.len(), 13);
    Ok(())
}

#[test]
fn preserves_maximum_length_utterance_id_and_its_longer_chunk_id()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = fixture(&[vec![(48000, 100)]])?;
    let id = "u".repeat(64);
    let mut plan: Value = serde_json::from_slice(&fs::read(&fixture.request.plan_path)?)?;
    plan["utterances"][0]["utterance_id"] = json!(id);
    fs::write(&fixture.request.plan_path, serde_json::to_vec(&plan)?)?;
    fixture.manifest["speech_plan"]["expected_utterance_ids"] = json!([id]);
    fixture.manifest["segments"][0]["utterance_id"] = json!(id);
    fixture.manifest["segments"][0]["chunk_id"] = json!(format!("{id}-chunk-0001"));
    fixture.save_manifest()?;
    let layout = verify_result_audio_layout(&fixture.request, &fixture.completed)?;
    assert_eq!(layout.utterances[0].utterance_id.as_str(), id);
    assert_eq!(
        layout.utterances[0].chunk_ids[0],
        format!("{id}-chunk-0001")
    );
    Ok(())
}

#[test]
fn covers_six_hundred_complete_utterances_without_512_limit()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(&vec![vec![(24000, 12)]; 600])?;
    let layout = verify_result_audio_layout(&fixture.request, &fixture.completed)?;
    assert_eq!(layout.utterances.len(), 600);
    assert_eq!(layout.frame_count, 14400);
    assert_eq!(layout.utterances[599].first_frame, 14376);
    assert_eq!(layout.utterances[599].utterance_id.as_str(), "line-599");
    Ok(())
}
