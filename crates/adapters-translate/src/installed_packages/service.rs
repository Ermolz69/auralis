use super::catalog;
use super::runtime::InstalledPackageRuntime;
use super::selection::PackageSelection;
use super::{TranslationPackageError, TranslationPackageStatus};
use crate::{ManagedLlamaRuntime, ManagedRuntimeConfig};
use auralis_translation_llamacpp::{
    ReleaseAsset, ReleaseManifest, hash_file, verify_runtime_files,
};
use ports::translation_model_runtime::TranslationModelRuntimePort;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::Duration;

const STATUS_SCHEMA_VERSION: u32 = 1;
const DEFAULT_CONTEXT_TOKENS: u32 = 2048;
const DEFAULT_GPU_LAYERS: u32 = 0;
const DEFAULT_STARTUP_TIMEOUT: Duration = Duration::from_secs(180);
const DEFAULT_READINESS_INTERVAL: Duration = Duration::from_millis(250);
const SERVER_EXECUTABLE_NAME: &str = "llama-server.exe";

pub struct TranslationPackageService {
    pub(super) packages_root: PathBuf,
    pub(super) cache_dir: PathBuf,
    pub(super) selection_file: PathBuf,
    install_lock: Mutex<()>,
}

impl TranslationPackageService {
    pub fn new(packages_root: PathBuf, cache_dir: PathBuf, selection_file: PathBuf) -> Self {
        Self {
            packages_root,
            cache_dir,
            selection_file,
            install_lock: Mutex::new(()),
        }
    }

    pub fn status(&self) -> Result<TranslationPackageStatus, TranslationPackageError> {
        let release = catalog::release()?;
        let installed = self.validate_structure(&release).is_ok();
        let selected =
            PackageSelection::load(&self.selection_file).is_ok_and(|value| value.is_some());
        Ok(TranslationPackageStatus {
            schema_version: STATUS_SCHEMA_VERSION,
            release_id: release.id,
            backend: catalog::BACKEND.to_owned(),
            supported: cfg!(all(target_os = "windows", target_arch = "x86_64")),
            installed,
            selected: selected && installed,
        })
    }

    pub fn install(&self) -> Result<TranslationPackageStatus, TranslationPackageError> {
        self.repair()
    }

    pub(super) fn lock_installation(&self) -> Result<MutexGuard<'_, ()>, TranslationPackageError> {
        match self.install_lock.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::WouldBlock) => Err(TranslationPackageError::Busy),
            Err(TryLockError::Poisoned(_)) => Err(TranslationPackageError::Invalid(
                "package installation lock is poisoned",
            )),
        }
    }

    pub fn select(
        &self,
        release_id: &str,
        backend: &str,
    ) -> Result<Arc<dyn TranslationModelRuntimePort>, TranslationPackageError> {
        self.ensure_supported_platform()?;
        let release = catalog::release()?;
        if release.id != release_id || backend != catalog::BACKEND {
            return Err(TranslationPackageError::Invalid(
                "requested package is not in the pinned catalog",
            ));
        }
        self.verify_complete(&release)?;
        let runtime = self.runtime_for(&release)?;
        PackageSelection::save(&self.selection_file)?;
        Ok(runtime)
    }

    pub fn selected_runtime(
        &self,
    ) -> Result<Option<Arc<dyn TranslationModelRuntimePort>>, TranslationPackageError> {
        let Some(_) = PackageSelection::load(&self.selection_file)? else {
            return Ok(None);
        };
        self.ensure_supported_platform()?;
        let release = catalog::release()?;
        self.validate_structure(&release)?;
        self.runtime_for(&release).map(Some)
    }

    fn runtime_for(
        &self,
        release: &ReleaseManifest,
    ) -> Result<Arc<dyn TranslationModelRuntimePort>, TranslationPackageError> {
        let root = self.package_root(release);
        let config = ManagedRuntimeConfig {
            executable: root.join("runtime").join(SERVER_EXECUTABLE_NAME),
            model_file: root.join("model").join(&release.model_asset().filename),
            context_tokens: DEFAULT_CONTEXT_TOKENS,
            gpu_layers: DEFAULT_GPU_LAYERS,
            startup_timeout: DEFAULT_STARTUP_TIMEOUT,
            readiness_interval: DEFAULT_READINESS_INTERVAL,
        };
        let variant = release
            .variants()
            .iter()
            .find(|variant| variant.backend == catalog::BACKEND)
            .ok_or(TranslationPackageError::Invalid(
                "pinned backend is missing",
            ))?
            .clone();
        Ok(Arc::new(InstalledPackageRuntime::new(
            root,
            variant,
            ManagedLlamaRuntime::new(config)?,
        )))
    }

    pub(super) fn verify_complete(
        &self,
        release: &ReleaseManifest,
    ) -> Result<(), TranslationPackageError> {
        verify_complete_at(&self.package_root(release), release)
    }

    fn validate_structure(&self, release: &ReleaseManifest) -> Result<(), TranslationPackageError> {
        validate_structure_at(&self.package_root(release), release)
    }

    pub(super) fn package_root(&self, release: &ReleaseManifest) -> PathBuf {
        self.packages_root.join(&release.id).join(catalog::BACKEND)
    }

    pub(super) fn ensure_supported_platform(&self) -> Result<(), TranslationPackageError> {
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Ok(())
        } else {
            Err(TranslationPackageError::Invalid(
                "this package supports Windows x64 only",
            ))
        }
    }
}

pub(super) fn verify_complete_at(
    root: &Path,
    release: &ReleaseManifest,
) -> Result<(), TranslationPackageError> {
    validate_structure_at(root, release)?;
    for asset in release.assets_for_backend(catalog::BACKEND)? {
        let path = asset_path(root, release, asset);
        let (digest, bytes) = hash_file(&path)
            .map_err(|_| TranslationPackageError::Invalid("installed asset could not be hashed"))?;
        if Some(bytes) != asset.bytes || !digest.to_string().eq_ignore_ascii_case(&asset.sha256) {
            return Err(TranslationPackageError::Invalid(
                "installed asset differs from its pinned digest",
            ));
        }
    }
    let variant = release
        .variants()
        .iter()
        .find(|variant| variant.backend == catalog::BACKEND)
        .ok_or(TranslationPackageError::Invalid(
            "pinned backend is missing",
        ))?;
    verify_runtime_files(&root.join("assets"), &root.join("runtime"), variant)?;
    Ok(())
}

fn validate_structure_at(
    root: &Path,
    release: &ReleaseManifest,
) -> Result<(), TranslationPackageError> {
    let metadata = std::fs::symlink_metadata(root)?;
    if !metadata.file_type().is_dir() {
        return Err(TranslationPackageError::Invalid(
            "installed package root is not a directory",
        ));
    }
    for (name, expected) in [
        ("release.json", catalog::RELEASE),
        ("profile.json", catalog::PROFILE),
    ] {
        let path = root.join(name);
        verify_regular_file(&path, Some(expected.len() as u64))?;
        if std::fs::read(path)? != expected {
            return Err(TranslationPackageError::Invalid(
                "installed manifest differs from pinned package",
            ));
        }
    }
    verify_regular_file(&root.join("runtime").join(SERVER_EXECUTABLE_NAME), None)?;
    for asset in release.assets_for_backend(catalog::BACKEND)? {
        verify_regular_file(&asset_path(root, release, asset), asset.bytes)?;
    }
    Ok(())
}

fn asset_path(root: &Path, release: &ReleaseManifest, asset: &ReleaseAsset) -> PathBuf {
    let directory = if asset.filename == release.model_asset().filename {
        "model"
    } else if asset.filename == release.model_notice().filename
        || asset.filename == release.runtime_notice().filename
    {
        "notices"
    } else {
        "assets"
    };
    root.join(directory).join(&asset.filename)
}

fn verify_regular_file(
    path: &Path,
    expected_bytes: Option<u64>,
) -> Result<(), TranslationPackageError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file()
        || expected_bytes.is_some_and(|length| metadata.len() != length)
    {
        return Err(TranslationPackageError::Invalid(
            "installed package contains a missing or changed file",
        ));
    }
    Ok(())
}
