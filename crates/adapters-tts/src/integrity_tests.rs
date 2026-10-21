use crate::{TtsRuntimeConfig, TtsWorkerClient};
use sha2::{Digest, Sha256};

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn client(root: &std::path::Path) -> Result<TtsWorkerClient, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(root.join("src/tts"))?;
    std::fs::write(root.join("src/tts/worker.py"), b"fixture")?;
    std::fs::write(root.join("metadata"), b"dependency fixture")?;
    std::fs::write(root.join("model"), b"model fixture")?;
    let receipt = serde_json::json!({"version":1,"package_files":[{"path":"src/tts/worker.py","sha256":digest(b"fixture")}],
        "dependencies":[{"path":root.join("metadata"),"sha256":digest(b"dependency fixture")}],
        "model_files":[{"engine":"voxcpm2","path":"model","sha256":digest(b"model fixture")}]}).to_string();
    std::fs::write(root.join("integrity.json"), &receipt)?;
    Ok(TtsWorkerClient::new(TtsRuntimeConfig {
        python_executable: root.join("python.exe"),
        package_root: root.into(),
        voice_store: root.into(),
        profile_root: root.into(),
        model_cache: root.into(),
        managed_root: root.into(),
        voxcpm_revision: "32279effe8c19989596f05d353d1447f51d9e915".into(),
        integrity_sha256: digest(receipt.as_bytes()),
    })?)
}
#[tokio::test]
async fn rejects_changed_model_and_package_before_any_worker_can_start()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let client = client(directory.path())?;
    let profile = directory.path().join("alternate-voxcpm2-profile.json");
    std::fs::write(&profile, br#"{"engine":"voxcpm2"}"#)?;
    client.verify_profile_integrity(&profile).await?;
    client.verify_integrity(Some("voxcpm2".into())).await?;
    std::fs::write(directory.path().join("model"), b"changed model")?;
    assert!(client.verify_profile_integrity(&profile).await.is_err());
    assert!(
        client
            .verify_integrity(Some("voxcpm2".into()))
            .await
            .is_err()
    );
    std::fs::write(directory.path().join("model"), b"model fixture")?;
    std::fs::write(
        directory.path().join("src/tts/worker.py"),
        b"changed package",
    )?;
    assert!(client.verify_integrity(None).await.is_err());
    std::fs::write(directory.path().join("src/tts/worker.py"), b"fixture")?;
    std::fs::write(
        directory.path().join("integrity.json"),
        b"changed inventory",
    )?;
    assert!(client.verify_integrity(None).await.is_err());
    Ok(())
}
