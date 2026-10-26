use super::{ManagedLlamaRuntime, RuntimePreparation, conflict, invalid, io_error, unexpected};
use auralis_translation::SourceHash;
use auralis_translation_llamacpp::{ModelProfile, PreparationControl, hash_file_with_control};
use ports::{PortError, translation_links::TranslationRunIntent};
use std::path::PathBuf;

struct ModelInput {
    profile: ModelProfile,
    path: PathBuf,
    expected_bytes: u64,
    expected_hash: SourceHash,
}

impl ManagedLlamaRuntime {
    pub(super) fn validate_model_input(
        &self,
        intent: &TranslationRunIntent,
        control: &RuntimePreparation,
    ) -> Result<(), PortError> {
        self.model_input(intent, control).map(|_| ())
    }

    fn model_input(
        &self,
        intent: &TranslationRunIntent,
        control: &RuntimePreparation,
    ) -> Result<ModelInput, PortError> {
        control.check().map_err(|cause| control.error(cause))?;
        let bytes = intent.profile_manifest_json.as_bytes();
        if SourceHash::digest(bytes).to_string() != intent.profile_sha256 {
            return Err(conflict(
                "frozen model profile digest differs from its bytes",
            ));
        }
        let profile =
            ModelProfile::from_json(bytes).map_err(|error| invalid(&error.to_string()))?;
        let expected_bytes = profile
            .model_file_bytes
            .ok_or_else(|| PortError::Unsupported {
                message: "managed execution requires a checked model profile".into(),
            })?;
        let minimum_context = profile
            .min_context_tokens
            .ok_or_else(|| invalid("checked model profile has no minimum context"))?;
        if self.config.context_tokens < minimum_context {
            return Err(invalid(
                "configured context is smaller than checked profile",
            ));
        }
        self.config.validate_files()?;
        let path = std::fs::canonicalize(&self.config.model_file).map_err(io_error)?;
        if std::fs::metadata(&path).map_err(io_error)?.len() != expected_bytes {
            return Err(conflict(
                "installed model size differs from checked profile",
            ));
        }
        let expected_hash = SourceHash::parse_hex(&profile.model_file_sha256)
            .ok_or_else(|| invalid("checked model digest is invalid"))?;
        control.check().map_err(|cause| control.error(cause))?;
        Ok(ModelInput {
            profile,
            path,
            expected_bytes,
            expected_hash,
        })
    }

    pub(super) async fn verify_model(
        &self,
        intent: &TranslationRunIntent,
        control: &RuntimePreparation,
    ) -> Result<(ModelProfile, PathBuf), PortError> {
        // Recheck metadata after native admission; preflight facts are not trusted for execution.
        let input = self.model_input(intent, control)?;
        let path_for_hash = input.path.clone();
        let hash_control = control.clone();
        let (actual_hash, actual_bytes) = tokio::task::spawn_blocking(move || {
            hash_file_with_control(&path_for_hash, &hash_control)
        })
        .await
        .map_err(|error| unexpected(&error.to_string()))?
        .map_err(|error| control.error(error))?;
        if actual_bytes != input.expected_bytes || actual_hash != input.expected_hash {
            return Err(conflict(
                "installed model digest differs from checked profile",
            ));
        }
        Ok((input.profile, input.path))
    }
}
