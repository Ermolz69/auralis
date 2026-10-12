use super::{paths::AppPaths, services::RuntimeServices};
use adapters_tts::{TtsRuntimeConfig, TtsWorkerClient};
use application::usecases::{
    speech::SpeechUseCases, speech_catalog::SpeechCatalogUseCase,
    translation::TranslationReviewUseCase,
};
use ports::{speech_catalog::SpeechCatalogPort, speech_workflow::SpeechExecutionPort};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledTtsRuntime {
    version: u8,
    python_executable: PathBuf,
    package_root: PathBuf,
    voice_store: PathBuf,
    profile_root: PathBuf,
    model_cache: PathBuf,
    voxcpm_revision: String,
    integrity_sha256: String,
}

fn load(paths: &AppPaths) -> Result<TtsWorkerClient, String> {
    let bytes = std::fs::read(paths.tts_runtime_config())
        .map_err(|e| format!("tts-runtime.json unavailable: {e}; run task setup:tts:local"))?;
    let config: InstalledTtsRuntime =
        serde_json::from_slice(&bytes).map_err(|e| format!("tts-runtime.json is invalid: {e}"))?;
    if config.version != 1 {
        return Err("Unsupported TTS runtime version".into());
    }
    for (label, path, file) in [
        ("Python", &config.python_executable, true),
        ("TTS package", &config.package_root, false),
        ("Voice store", &config.voice_store, false),
        ("Profiles", &config.profile_root, false),
        ("Model cache", &config.model_cache, false),
    ] {
        if !path.is_absolute() || (file && !path.is_file()) || (!file && !path.is_dir()) {
            return Err(format!(
                "{label} is missing or is not an absolute path: {}",
                path.display()
            ));
        }
    }
    TtsWorkerClient::new(TtsRuntimeConfig {
        python_executable: config.python_executable,
        package_root: config.package_root,
        voice_store: config.voice_store,
        profile_root: config.profile_root,
        model_cache: config.model_cache,
        managed_root: paths
            .projects()
            .canonicalize()
            .map_err(|e| format!("TTS managed storage is unavailable: {e}"))?,
        voxcpm_revision: config.voxcpm_revision,
        integrity_sha256: config.integrity_sha256,
    })
    .map_err(|e| e.to_string())
}

pub fn setup(
    paths: &AppPaths,
    services: &RuntimeServices,
    runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    review: Arc<TranslationReviewUseCase>,
    subtitles: Arc<application::usecases::translation::InspectTranslationSourceUseCase>,
    emitter: Arc<dyn Fn(ports::job_scheduler::JobLifecycleEvent) + Send + Sync>,
) -> (Arc<SpeechCatalogUseCase>, Arc<SpeechUseCases>) {
    let loaded = load(paths).map(Arc::new);
    let runtime_error = loaded.as_ref().err().cloned();
    let catalog = loaded
        .as_ref()
        .ok()
        .map(|client| client.clone() as Arc<dyn SpeechCatalogPort>);
    let engine = loaded
        .ok()
        .map(|client| client as Arc<dyn SpeechExecutionPort>);
    let catalog_cases = SpeechCatalogUseCase::new(services.project_repo.clone(), catalog.clone())
        .with_unavailable_reason(
            runtime_error
                .clone()
                .unwrap_or_else(|| "TTS runtime unavailable".into()),
        );
    let cases = SpeechUseCases {
        projects: services.project_repo.clone(),
        store: services.speech.clone(),
        engine,
        catalog,
        runtime_error,
        runtime,
        artifacts: services.artifact_store.clone(),
        index: services.artifact_index.clone(),
        translation_review: review,
        subtitles,
        emitter,
    };
    (Arc::new(catalog_cases), Arc::new(cases))
}
