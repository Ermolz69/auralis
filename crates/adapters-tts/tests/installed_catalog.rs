use adapters_tts::{TtsRuntimeConfig, TtsWorkerClient};
use domain::project::ProjectId;
use ports::speech_catalog::SpeechCatalogPort;
use std::path::PathBuf;

const VOX_REVISION: &str = "32279effe8c19989596f05d353d1447f51d9e915";

#[tokio::test]
#[ignore = "requires an installed TTS worker and the user-selected Ciri/Geralt voice copies"]
async fn installed_worker_lists_ciri_and_geralt_for_both_engines()
-> Result<(), Box<dyn std::error::Error>> {
    let required_path = |name| -> Result<PathBuf, std::io::Error> {
        std::env::var_os(name)
            .map(PathBuf::from)
            .ok_or_else(|| std::io::Error::other(format!("Missing required {name}")))
    };
    let client = TtsWorkerClient::new(TtsRuntimeConfig {
        python_executable: required_path("AURALIS_TEST_TTS_PYTHON")?,
        package_root: required_path("AURALIS_TEST_TTS_PACKAGE_ROOT")?,
        voice_store: required_path("AURALIS_TEST_TTS_VOICE_STORE")?,
        profile_root: required_path("AURALIS_TEST_TTS_PROFILE_ROOT")?,
        model_cache: required_path("AURALIS_TEST_TTS_MODEL_CACHE")?,
        managed_root: std::env::temp_dir(),
        voxcpm_revision: VOX_REVISION.into(),
        integrity_sha256: std::env::var("AURALIS_TEST_TTS_INTEGRITY_SHA256")?,
    })?;
    let snapshot = client.snapshot(&ProjectId::new()).await?;
    assert_eq!(snapshot.profiles.len(), 2);
    for (engine, profile) in [
        ("voxcpm2", "native-plan-voxcpm2"),
        ("espeech", "native-plan-espeech"),
    ] {
        assert!(
            snapshot
                .profiles
                .iter()
                .any(|item| { item.engine_id == engine && item.profile_id == profile })
        );
        for character in ["ciri", "geralt"] {
            let voice_id = format!("{character}_{engine}");
            let voice = snapshot
                .voices
                .iter()
                .find(|item| item.voice_id == voice_id)
                .ok_or_else(|| std::io::Error::other("Selected voice missing in TTS catalog"))?;
            assert!(voice.references.iter().any(|item| {
                item.engine_id == engine
                    && item.emotion_id == "neutral"
                    && item.audio_sha256.len() == 64
            }));
        }
    }
    Ok(())
}
