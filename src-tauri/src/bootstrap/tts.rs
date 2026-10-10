use adapters_tts::{TtsRuntimeConfig, TtsWorkerClient};
use application::usecases::speech_catalog::SpeechCatalogUseCase;
use ports::repository::ProjectRepository;
use ports::speech_catalog::SpeechCatalogPort;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

use super::paths::AppPaths;

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
}

pub fn setup(paths: &AppPaths, projects: Arc<dyn ProjectRepository>) -> Arc<SpeechCatalogUseCase> {
    let catalog = std::fs::read(paths.tts_runtime_config())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<InstalledTtsRuntime>(&bytes).ok())
        .filter(|config| {
            config.version == 1
                && config.python_executable.is_file()
                && config.package_root.join("src/tts/worker.py").is_file()
                && config.voice_store.is_dir()
                && config.profile_root.is_dir()
                && config.model_cache.is_dir()
        })
        .and_then(|config| {
            TtsWorkerClient::new(TtsRuntimeConfig {
                python_executable: config.python_executable,
                package_root: config.package_root,
                voice_store: config.voice_store,
                profile_root: config.profile_root,
                model_cache: config.model_cache,
                managed_root: paths.projects(),
                voxcpm_revision: config.voxcpm_revision,
            })
            .ok()
        })
        .map(|client| Arc::new(client) as Arc<dyn SpeechCatalogPort>);
    Arc::new(SpeechCatalogUseCase::new(projects, catalog))
}
