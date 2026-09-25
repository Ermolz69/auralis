use super::{TranslationPackageError, catalog};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;
const MAX_SELECTION_BYTES: u64 = 4096;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackageSelection {
    schema_version: u32,
    release_id: String,
    backend: String,
}

impl PackageSelection {
    pub(super) fn load(path: &Path) -> Result<Option<Self>, TranslationPackageError> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata)
                if metadata.file_type().is_file() && metadata.len() <= MAX_SELECTION_BYTES =>
            {
                metadata
            }
            Ok(_) => {
                return Err(TranslationPackageError::Invalid(
                    "selection marker is not a regular bounded file",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let bytes = std::fs::read(path)?;
        if bytes.len() as u64 != metadata.len() {
            return Err(TranslationPackageError::Invalid(
                "selection marker changed during reading",
            ));
        }
        let selected: Self = serde_json::from_slice(&bytes)?;
        selected.validate()?;
        Ok(Some(selected))
    }

    pub(super) fn save(path: &Path) -> Result<(), TranslationPackageError> {
        let release = catalog::release()?;
        let marker = Self {
            schema_version: SCHEMA_VERSION,
            release_id: release.id,
            backend: catalog::BACKEND.to_owned(),
        };
        let parent = path.parent().ok_or(TranslationPackageError::Invalid(
            "selection path has no parent directory",
        ))?;
        std::fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(
            ".translation-package-selection-{}.tmp",
            uuid::Uuid::new_v4()
        ));
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&serde_json::to_vec(&marker)?)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok::<(), TranslationPackageError>(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        result
    }

    pub(super) fn matches_catalog(&self) -> bool {
        catalog::release().is_ok_and(|release| {
            self.schema_version == SCHEMA_VERSION
                && self.release_id == release.id
                && self.backend == catalog::BACKEND
        })
    }

    fn validate(&self) -> Result<(), TranslationPackageError> {
        if !self.matches_catalog() {
            return Err(TranslationPackageError::Invalid(
                "selection marker does not match the pinned package",
            ));
        }
        Ok(())
    }
}
