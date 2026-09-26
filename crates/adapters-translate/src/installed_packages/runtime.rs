use crate::ManagedLlamaRuntime;
use async_trait::async_trait;
use auralis_translation_llamacpp::{RuntimeVariant, verify_runtime_files};
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};
use std::path::PathBuf;

pub(super) struct InstalledPackageRuntime {
    root: PathBuf,
    variant: RuntimeVariant,
    runtime: ManagedLlamaRuntime,
}

impl InstalledPackageRuntime {
    pub(super) fn new(
        root: PathBuf,
        variant: RuntimeVariant,
        runtime: ManagedLlamaRuntime,
    ) -> Self {
        Self {
            root,
            variant,
            runtime,
        }
    }
}

#[async_trait]
impl TranslationModelRuntimePort for InstalledPackageRuntime {
    async fn acquire(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        let root = self.root.clone();
        let variant = self.variant.clone();
        tokio::task::spawn_blocking(move || {
            verify_runtime_files(&root.join("assets"), &root.join("runtime"), &variant)
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
        .map_err(|error| PortError::Conflict {
            resource: "installed translation runtime".into(),
            message: error.to_string(),
        })?;
        self.runtime.acquire(intent).await
    }
}
