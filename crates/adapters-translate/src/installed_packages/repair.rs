use super::catalog;
use super::service::verify_complete_at;
use super::{TranslationPackageError, TranslationPackageService, TranslationPackageStatus};
use auralis_translation_llamacpp::{ReleaseManifest, download_release_assets, install_offline};
use std::path::Path;

const PREVIOUS_PACKAGE_DIRECTORY: &str = ".previous-cpu";

impl TranslationPackageService {
    pub(super) fn repair(&self) -> Result<TranslationPackageStatus, TranslationPackageError> {
        self.ensure_supported_platform()?;
        let _guard = self.lock_installation()?;
        let release = catalog::release()?;
        let final_root = self.package_root(&release);
        let release_root = final_root.parent().ok_or(TranslationPackageError::Invalid(
            "package release path has no parent directory",
        ))?;
        ensure_regular_directory(&self.packages_root)?;
        ensure_regular_directory(release_root)?;
        let previous = release_root.join(PREVIOUS_PACKAGE_DIRECTORY);
        recover_previous(&final_root, &previous, &release)?;
        if self.verify_complete(&release).is_ok() {
            return self.status();
        }

        download_release_assets(
            catalog::RELEASE,
            catalog::PROFILE,
            catalog::BACKEND,
            &self.cache_dir,
        )?;
        let workspace = tempfile::Builder::new()
            .prefix(".repair-")
            .tempdir_in(&self.packages_root)?;
        let candidate = install_offline(
            catalog::RELEASE,
            catalog::PROFILE,
            catalog::BACKEND,
            &self.cache_dir,
            workspace.path(),
        )?;
        verify_complete_at(&candidate.root, &release)?;

        if directory_exists(&previous)? {
            std::fs::remove_dir_all(&previous)?;
        }
        let replaced = directory_exists(&final_root)?;
        if replaced {
            std::fs::rename(&final_root, &previous)?;
        }
        if let Err(error) = std::fs::rename(&candidate.root, &final_root) {
            if replaced && std::fs::rename(&previous, &final_root).is_err() {
                return Err(TranslationPackageError::Invalid(
                    "package repair failed and the previous package needs recovery",
                ));
            }
            return Err(error.into());
        }
        if replaced {
            std::fs::remove_dir_all(&previous)?;
        }
        self.status()
    }
}

fn recover_previous(
    final_root: &Path,
    previous: &Path,
    release: &ReleaseManifest,
) -> Result<(), TranslationPackageError> {
    if !directory_exists(previous)? {
        return Ok(());
    }
    if !directory_exists(final_root)? {
        std::fs::rename(previous, final_root)?;
    } else if verify_complete_at(final_root, release).is_ok() {
        std::fs::remove_dir_all(previous)?;
    }
    Ok(())
}

fn ensure_regular_directory(path: &Path) -> Result<(), TranslationPackageError> {
    if !path.is_absolute() {
        return Err(TranslationPackageError::Invalid(
            "package storage path must be absolute",
        ));
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => return Ok(()),
        Ok(_) => {
            return Err(TranslationPackageError::Invalid(
                "package storage path is not a regular directory",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    std::fs::create_dir(path)?;
    Ok(())
}

fn directory_exists(path: &Path) -> Result<bool, TranslationPackageError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(true),
        Ok(_) => Err(TranslationPackageError::Invalid(
            "package path is not a regular directory",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
