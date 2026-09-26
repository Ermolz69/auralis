use adapters_translate::{ManagedLlamaRuntime, ManagedRuntimeConfig};
use auralis_translation::SourceHash;
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::TranslationModelRuntimePort;
use std::error::Error;
use std::time::Duration;
use uuid::Uuid;

const UNCHECKED_PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);
const CHECKED_PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);

fn intent(profile: &str) -> TranslationRunIntent {
    TranslationRunIntent {
        run_id: Uuid::new_v4(),
        translation_id: Uuid::new_v4(),
        profile_manifest_json: profile.into(),
        profile_sha256: SourceHash::digest(profile.as_bytes()).to_string(),
        max_target_segments: 8,
        context_before_segments: 0,
        context_after_segments: 0,
        parser_version: 1,
        policy_fingerprint: "fixture-policy".into(),
        created_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn rejects_unchecked_or_mismatched_model_before_process_start() -> Result<(), Box<dyn Error>>
{
    let directory = tempfile::tempdir()?;
    let executable = directory.path().join("llama-server.exe");
    let model = directory.path().join("model.gguf");
    std::fs::write(&executable, b"not executable")?;
    std::fs::write(&model, b"not the checked model")?;
    let runtime = ManagedLlamaRuntime::new(ManagedRuntimeConfig {
        executable,
        model_file: model,
        context_tokens: 2048,
        gpu_layers: 0,
        ram_cache_mib: adapters_translate::DEFAULT_RAM_CACHE_MIB,
        startup_timeout: Duration::from_secs(1),
        readiness_interval: Duration::from_millis(100),
    })?;
    assert!(matches!(
        runtime.acquire(&intent(UNCHECKED_PROFILE)).await,
        Err(PortError::Unsupported { .. })
    ));
    assert!(matches!(
        runtime.acquire(&intent(CHECKED_PROFILE)).await,
        Err(PortError::Conflict { .. })
    ));
    let mut invalid_digest = intent(CHECKED_PROFILE);
    invalid_digest.profile_sha256 = "0".repeat(64);
    assert!(matches!(
        runtime.acquire(&invalid_digest).await,
        Err(PortError::Conflict { .. })
    ));
    std::fs::remove_file(directory.path().join("model.gguf"))?;
    assert!(matches!(
        runtime.acquire(&intent(CHECKED_PROFILE)).await,
        Err(PortError::NotFound { .. })
    ));
    Ok(())
}

#[test]
fn rejects_oversized_ram_cache_before_model_admission() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    for cache_mib in [
        adapters_translate::DEFAULT_RAM_CACHE_MIB,
        128,
        512,
        513,
        u32::MAX,
    ] {
        let runtime = ManagedLlamaRuntime::new(ManagedRuntimeConfig {
            executable: directory.path().join("llama-server.exe"),
            model_file: directory.path().join("model.gguf"),
            context_tokens: 2048,
            gpu_layers: 0,
            ram_cache_mib: cache_mib,
            startup_timeout: Duration::from_secs(1),
            readiness_interval: Duration::from_millis(100),
        });
        if cache_mib <= 512 {
            assert!(runtime.is_ok());
        } else {
            assert!(matches!(runtime, Err(PortError::InvalidSource { .. })));
        }
    }
    Ok(())
}
