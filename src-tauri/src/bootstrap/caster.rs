use adapters_caster::{CasterRuntimeConfig, CasterWorkerAdapter};
use application::usecases::caster::CasterUseCases;
use application::usecases::project::lifecycle::ProjectLifecycleLocks;
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::paths::AppPaths;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledCasterRuntime {
    version: u8,
    python_executable: PathBuf,
    voice_store: PathBuf,
    caster_version: String,
    contract_version: String,
    wheel_manifest_sha256: String,
}

pub fn verified_manifest(python_executable: &Path, expected: &str) -> bool {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return false;
    }
    let Some(install_root) = python_executable.parent().and_then(|path| path.parent()) else {
        return false;
    };
    let Ok(manifest) = std::fs::read(install_root.join("caster-wheelhouse-manifest.json")) else {
        return false;
    };
    format!("{:x}", Sha256::digest(manifest)) == expected
}

pub fn setup(
    paths: &AppPaths,
    projects: Arc<dyn ProjectRepository>,
    artifacts: Arc<dyn ArtifactIndex>,
    locks: Arc<ProjectLifecycleLocks>,
) -> (Arc<CasterUseCases>, Option<CasterRuntimeConfig>) {
    let runtime = std::fs::read(paths.caster_runtime_config())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<InstalledCasterRuntime>(&bytes).ok())
        .and_then(|config| {
            if config.version != 1
                || config.caster_version != "0.2.0"
                || config.contract_version != "0.2.0"
                || !config.python_executable.is_absolute()
                || !config.python_executable.is_file()
                || !config.voice_store.is_absolute()
                || !verified_manifest(&config.python_executable, &config.wheel_manifest_sha256)
            {
                return None;
            }
            Some(CasterRuntimeConfig {
                python_executable: config.python_executable,
                voice_store: config.voice_store,
            })
        });
    (
        Arc::new(CasterUseCases::new(
            projects,
            artifacts,
            Arc::new(CasterWorkerAdapter::new(runtime.clone(), paths.projects())),
            locks,
        )),
        runtime,
    )
}
