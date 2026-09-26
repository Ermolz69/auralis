use adapters_translate::{ManagedLlamaRuntime, ManagedRuntimeConfig};
use auralis_translation::SourceHash;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::TranslationModelRuntimePort;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.checked.experimental.json"
);
const SERVER_ENV: &str = "AURALIS_TEST_LLAMA_SERVER";
const MODEL_ENV: &str = "AURALIS_TEST_GGUF";

#[tokio::test]
#[ignore = "requires an explicitly installed checked llama.cpp binary and GGUF"]
async fn starts_and_releases_the_checked_local_server() -> Result<(), Box<dyn Error>> {
    let executable = PathBuf::from(std::env::var_os(SERVER_ENV).ok_or(SERVER_ENV)?);
    let model_file = PathBuf::from(std::env::var_os(MODEL_ENV).ok_or(MODEL_ENV)?);
    let runtime = ManagedLlamaRuntime::new(ManagedRuntimeConfig {
        executable,
        model_file,
        context_tokens: 2048,
        gpu_layers: 99,
        ram_cache_mib: adapters_translate::DEFAULT_RAM_CACHE_MIB,
        startup_timeout: Duration::from_secs(120),
        readiness_interval: Duration::from_millis(250),
    })?;
    let intent = TranslationRunIntent {
        run_id: Uuid::new_v4(),
        translation_id: Uuid::new_v4(),
        profile_manifest_json: PROFILE.into(),
        profile_sha256: SourceHash::digest(PROFILE.as_bytes()).to_string(),
        max_target_segments: 8,
        context_before_segments: 0,
        context_after_segments: 0,
        parser_version: 1,
        policy_fingerprint: "runtime-smoke".into(),
        created_at: chrono::Utc::now(),
    };
    let lease = runtime.acquire(&intent).await?;
    let props: serde_json::Value = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()?
        .get(format!("{}props", lease.server_url()))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(props["total_slots"], 1);
    let address = lease
        .server_url()
        .strip_prefix("http://")
        .and_then(|value| value.strip_suffix('/'))
        .ok_or("unexpected loopback server URL")?
        .to_owned();
    assert!(std::net::TcpStream::connect(&address).is_ok());
    drop(lease);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if std::net::TcpStream::connect(&address).is_err() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("managed server remained reachable after lease release".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}
