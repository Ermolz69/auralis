#[path = "audio_assembly/support.rs"]
mod support;

use adapters_ffmpeg::assembly::{AudioAssemblyLimits, FfmpegAudioAssembler};
use ports::{audio_assembly::AudioAssemblyPort, cancellation::CancellationToken};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use support::*;

fn assert_source(root: &Path, source: &str) {
    assert_eq!(source.len(), 40);
    assert!(source.bytes().all(|byte| byte.is_ascii_hexdigit()));
    for refs in [vec![source, "HEAD"], vec!["HEAD"]] {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(root)
            .args(["diff", "--quiet"])
            .args(refs)
            .args([
                "--",
                "crates",
                "Cargo.toml",
                "Cargo.lock",
                "Taskfile.yml",
                "taskfiles/rust.yml",
                "tools/revoice/prepare-audio-fixtures.mjs",
            ]);
        assert!(
            matches!(command.status(), Ok(status) if status.success()),
            "Production code differs from the frozen committed candidate"
        );
    }
}

fn write_receipt(path: &Path, value: &serde_json::Value) {
    let bytes = match serde_json::to_vec_pretty(value) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert!(fs::write(path, bytes).is_ok());
}

fn generated_bytes(path: &Path) -> u64 {
    let entries = match fs::read_dir(path) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    entries
        .map(|entry| {
            let entry = match entry {
                Ok(value) => value,
                Err(error) => panic!("{error}"),
            };
            let metadata = match entry.metadata() {
                Ok(value) => value,
                Err(error) => panic!("{error}"),
            };
            if metadata.is_dir() {
                generated_bytes(&entry.path())
            } else {
                metadata.len()
            }
        })
        .sum()
}

#[tokio::test]
#[ignore = "Explicit frozen, admitted managed-FFmpeg fixture matrix; never a native or speech quality gate"]
async fn ffmpeg_matrix() {
    let root = match Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
    {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    let spec_path = match std::env::var_os("AURALIS_FFMPEG_ASSEMBLY_SPEC") {
        Some(value) => PathBuf::from(value),
        None => panic!("Use task revoice:audio:real SPEC=..."),
    };
    let mut bytes = Vec::new();
    let file = match File::open(&spec_path) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert!(file.take(1_048_577).read_to_end(&mut bytes).is_ok());
    assert!(bytes.len() <= 1_048_576);
    let spec: Spec = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert_eq!(spec.version, 1);
    assert_eq!(spec.ffmpeg.version, "8.1.2");
    assert_eq!(spec.cases.len(), 3);
    assert_eq!(spec.budgets.max_invocations_per_case, 1);
    assert_eq!(spec.budgets.max_generated_bytes, 64 * 1024 * 1024);
    assert!(
        spec.budgets.process_seconds <= 60
            && spec.budgets.total_seconds <= 180
            && spec.budgets.cleanup_seconds <= 30
    );
    assert!(!spec.node_version.is_empty() && spec.scope.contains("Synthetic"));
    assert_source(&root, &spec.source_commit);
    assert_eq!(
        hash_file(&root.join("tools/revoice/prepare-audio-fixtures.mjs")),
        spec.generator_sha256
    );
    let fixture_root = match root.join(&spec.fixture_root).canonicalize() {
        Ok(value) => value,
        Err(error) => panic!("{error}"),
    };
    assert!(fixture_root.starts_with(&root));
    for fixture in &spec.fixtures {
        let source = match fixture_root.join(&fixture.file).canonicalize() {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        };
        assert!(source.starts_with(&fixture_root));
        assert_eq!(hash_file(&source), fixture.sha256);
    }
    let spec_hash = hash_file(&spec_path);
    for case in &spec.cases {
        assert!(
            !case.case_id.is_empty()
                && case.case_id.len() <= 64
                && case
                    .case_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        );
        let directory = root.join("target/revoice").join(&case.case_id);
        assert!(
            fs::create_dir(&directory).is_ok(),
            "Frozen case attempt already exists: {}",
            case.case_id
        );
        let staging = directory.join("staging");
        assert!(fs::create_dir(&staging).is_ok());
        let receipt_path = directory.join("receipt.json");
        let mut receipt = serde_json::json!({ "version": 1, "case_id": case.case_id,
            "source_commit": spec.source_commit, "spec_sha256": spec_hash, "ffmpeg_sha256": spec.ffmpeg.sha256,
            "scope": spec.scope, "status": "started", "expected_frames": case.expected_frames,
            "expected_pcm_sha256": case.expected_pcm_sha256, "voices_changed": false, "models_loaded": false });
        write_receipt(&receipt_path, &receipt);
        let limits = AudioAssemblyLimits {
            max_input_bytes: spec.budgets.max_generated_bytes,
            max_pcm_bytes: spec.budgets.max_generated_bytes,
            process_timeout: Duration::from_secs(spec.budgets.process_seconds),
            total_timeout: Duration::from_secs(spec.budgets.total_seconds),
            ..AudioAssemblyLimits::default()
        };
        let assembler = match FfmpegAudioAssembler::new(
            root.join(&spec.ffmpeg.file),
            digest(&spec.ffmpeg.sha256),
            fixture_root.clone(),
            staging,
            limits,
        ) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        };
        let started = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(spec.budgets.total_seconds + spec.budgets.cleanup_seconds),
            assembler.assemble(
                request(&spec, case, &fixture_root, &spec_hash),
                CancellationToken::new(),
            ),
        )
        .await;
        receipt["elapsed_ms"] = serde_json::json!(started.elapsed().as_millis());
        match outcome {
            Ok(Ok(result)) => {
                receipt["status"] = serde_json::json!("completed");
                receipt["wav"] = serde_json::json!(result.wav_path);
                receipt["wav_sha256"] = serde_json::json!(result.wav_sha256);
                receipt["assembly_manifest"] = serde_json::json!(result.manifest_path);
                receipt["assembly_manifest_sha256"] = serde_json::json!(result.manifest_sha256);
                receipt["frames"] = serde_json::json!(result.frames);
                receipt["channels"] = serde_json::json!(result.channels);
                receipt["pcm_sha256"] = serde_json::json!(result.pcm_sha256);
                receipt["peak_sample_abs"] = serde_json::json!(result.peak_sample_abs);
                receipt["full_scale_samples"] = serde_json::json!(result.full_scale_samples);
                receipt["admission_guardian_pid"] =
                    serde_json::json!(result.admission_guardian_pid);
                receipt["admission_release_confirmed"] = serde_json::json!(true);
                let generated = generated_bytes(&directory);
                receipt["generated_bytes_before_final_receipt"] = serde_json::json!(generated);
                let matches = result.frames == case.expected_frames
                    && result.pcm_sha256.as_str() == case.expected_pcm_sha256
                    && result.peak_sample_abs == case.expected_peak
                    && result.full_scale_samples == case.expected_full_scale
                    && generated < spec.budgets.max_generated_bytes - 1_048_576;
                receipt["independent_expected_pcm_matched"] = serde_json::json!(matches);
                if !matches {
                    receipt["status"] = serde_json::json!("failed_verification");
                }
                write_receipt(&receipt_path, &receipt);
                assert!(
                    matches,
                    "Independent frozen PCM oracle failed: {}",
                    case.case_id
                );
                assert_eq!(hash_file(&result.wav_path), result.wav_sha256.as_str());
                assert_eq!(
                    hash_file(&result.manifest_path),
                    result.manifest_sha256.as_str()
                );
            }
            other => {
                receipt["status"] = serde_json::json!("failed");
                receipt["error"] = serde_json::json!(format!("{other:?}"));
                write_receipt(&receipt_path, &receipt);
                panic!(
                    "Frozen FFmpeg case failed; stop without retries: {}: {other:?}",
                    case.case_id
                );
            }
        }
        println!("Retained {}", receipt_path.display());
    }
}
