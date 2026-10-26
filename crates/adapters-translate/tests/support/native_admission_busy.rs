use super::*;
use ports::cancellation::CancellationToken;

#[tokio::test]
#[ignore = "requires one available native model-free machine lease; no model or server is loaded"]
async fn invalid_inputs_precede_admission_but_valid_input_keeps_the_busy_guard()
-> Result<(), Box<dyn Error>> {
    let _serial = NATIVE_ADMISSION.lock().await;
    let directory = tempfile::tempdir()?;
    let executable = directory.path().join("llama-server.exe");
    let model = directory.path().join("model.gguf");
    let fixture_bytes = b"model-free metadata fixture";
    std::fs::write(&executable, b"not executable")?;
    std::fs::write(&model, fixture_bytes)?;
    let runtime = ManagedLlamaRuntime::new(ManagedRuntimeConfig {
        executable,
        model_file: model,
        context_tokens: 2048,
        gpu_layers: 0,
        ram_cache_mib: adapters_translate::DEFAULT_RAM_CACHE_MIB,
        startup_timeout: Duration::from_secs(1),
        readiness_interval: Duration::from_millis(100),
    })?;
    let mut profile: serde_json::Value = serde_json::from_str(CHECKED_PROFILE)?;
    profile["model_file_bytes"] = serde_json::json!(fixture_bytes.len());
    profile["model_file_sha256"] = serde_json::json!(SourceHash::digest(fixture_bytes).to_string());
    let valid = intent(&profile.to_string());
    let mut changed_digest = valid.clone();
    changed_digest.profile_sha256 = "0".repeat(64);
    let cancellation = CancellationToken::new();
    cancellation.cancel();

    let lease = common::model_admission::ModelAdmissionLease::acquire_model_free()?;
    let cancelled = runtime
        .acquire_with_control(&valid, cancellation)
        .await
        .err();
    let unchecked = runtime.acquire(&intent(UNCHECKED_PROFILE)).await.err();
    let wrong_size = runtime.acquire(&intent(CHECKED_PROFILE)).await.err();
    let invalid_digest = runtime.acquire(&changed_digest).await.err();
    let busy = runtime.acquire(&valid).await.err();
    lease.release()?;

    assert!(
        matches!(cancelled, Some(PortError::Cancelled)),
        "{cancelled:?}"
    );
    assert!(
        matches!(unchecked, Some(PortError::Unsupported { .. })),
        "{unchecked:?}"
    );
    assert!(
        matches!(wrong_size, Some(PortError::Conflict { .. })),
        "{wrong_size:?}"
    );
    assert!(
        matches!(invalid_digest, Some(PortError::Conflict { .. })),
        "{invalid_digest:?}"
    );
    assert!(matches!(busy, Some(PortError::Busy { .. })), "{busy:?}");
    Ok(())
}
