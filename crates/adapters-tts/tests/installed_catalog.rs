use adapters_tts::{TtsRuntimeConfig, TtsWorkerClient};
use domain::project::ProjectId;
use ports::speech_catalog::SpeechCatalogPort;
use std::path::PathBuf;

const VOX_REVISION: &str = "32279effe8c19989596f05d353d1447f51d9e915";

#[tokio::test]
#[ignore = "requires an installed TTS worker and the eight new voice manifests"]
async fn installed_worker_lists_new_voices_for_both_engines()
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
    assert_eq!(snapshot.voices.len(), 8);
    assert!(
        snapshot
            .voices
            .iter()
            .all(|voice| voice.voice_id != "ciri" && voice.voice_id != "geralt")
    );
    let emotions = [
        "neutral",
        "joy",
        "sadness",
        "anger",
        "fear",
        "surprise",
        "question",
        "whisper",
        "mysterious",
        "serious",
        "excited",
        "disgust",
    ];
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
        for voice_id in [
            "marina_elevenlabs_ru",
            "alex_analyst_elevenlabs_ru",
            "anastasiya_kim_elevenlabs_ru",
            "artur_denophine_elevenlabs_ru",
            "denis_elevenlabs_ru",
            "nastya_elevenlabs_ru",
            "rina_elevenlabs_ru",
            "stanislav_elevenlabs_ru",
        ] {
            let voice = snapshot
                .voices
                .iter()
                .find(|item| item.voice_id == voice_id)
                .ok_or_else(|| std::io::Error::other("Selected voice missing in TTS catalog"))?;
            assert_eq!(voice.references.len(), 24);
            for emotion in emotions {
                assert!(voice.references.iter().any(|item| {
                    item.engine_id == engine
                        && item.emotion_id == emotion
                        && item.audio_sha256.len() == 64
                        && item.reference_id == format!("ref-{engine}-{emotion}")
                        && matches!(
                            item.qualification_status.as_str(),
                            "qualified" | "unverified" | "rejected"
                        )
                }));
            }
        }
    }
    Ok(())
}
