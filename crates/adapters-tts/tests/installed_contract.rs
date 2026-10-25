use adapters_tts::{TtsRuntimeConfig, TtsWorkerClient};
use domain::project::ProjectId;
use ports::{
    puza_contract::{PuzaContractPort, PuzaVoiceBinding},
    speech_catalog::SpeechCatalogPort,
};

#[tokio::test]
#[ignore = "requires the pinned local CONTRACT and human-qualified voice copies; no model loading"]
async fn installed_contract_checks_empty_projects_sources_and_all_four_voice_assignments()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let data = std::env::var("AURALIS_TEST_PUZA_RUNTIME")?;
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(data)?)?;
    let path = |name: &str| -> Result<std::path::PathBuf, std::io::Error> {
        value[name]
            .as_str()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| std::io::Error::other(format!("Missing {name}")))
    };
    let client = TtsWorkerClient::new(TtsRuntimeConfig {
        python_executable: path("python_executable")?,
        package_root: path("package_root")?,
        voice_store: path("voice_store")?,
        profile_root: path("profile_root")?,
        model_cache: path("model_cache")?,
        managed_root: directory.path().into(),
        voxcpm_revision: value["voxcpm_revision"]
            .as_str()
            .ok_or("missing revision")?
            .into(),
        integrity_sha256: value["integrity_sha256"]
            .as_str()
            .ok_or("missing hash")?
            .into(),
    })?;
    let project = ProjectId::new();
    let empty = client.prepare_project(&project, None, Vec::new()).await?;
    assert_eq!(empty.project_json, r#"{"documents":[],"version":1}"#);
    assert_eq!(empty.contract_version, "0.2.0");
    let catalog = client.snapshot(&project).await?;
    assert_eq!(catalog.voices.len(), 4);
    let mut qualified_checks = 0;
    for voice in &catalog.voices {
        for reference in voice
            .references
            .iter()
            .filter(|reference| reference.qualification_status == "qualified")
        {
            qualified_checks += 1;
            let binding = PuzaVoiceBinding {
                technical_reference_permission: None,
                role_id: "narrator".into(),
                display_name: "Рассказчик".into(),
                voice_id: voice.voice_id.clone(),
                reference_id: reference.reference_id.clone(),
                engine_id: reference.engine_id.clone(),
            };
            let snapshot = client
                .prepare_project(
                    &project,
                    Some("Проверенный русский текст.".into()),
                    vec![binding.clone()],
                )
                .await?;
            assert!(snapshot.cast_json.contains(&voice.voice_id));
            assert_eq!(
                snapshot.source_text.as_deref(),
                Some("Проверенный русский текст.")
            );
            let mut wrong = binding;
            wrong.reference_id = "missing-reference".into();
            assert!(
                client
                    .prepare_project(&project, None, vec![wrong])
                    .await
                    .is_err()
            );
        }
    }
    assert_eq!(qualified_checks, 4);
    assert!(
        client
            .prepare_project(&project, Some(String::new()), Vec::new())
            .await
            .is_err()
    );
    Ok(())
}
