use crate::ManagedLlamaRuntime;
use crate::managed_runtime::preparation::RuntimePreparation;
use async_trait::async_trait;
use auralis_translation_llamacpp::{RuntimeVariant, verify_runtime_files_with_control};
use ports::cancellation::CancellationToken;
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
    async fn acquire_with_control(
        &self,
        intent: &TranslationRunIntent,
        cancellation: CancellationToken,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        let cancel_on_drop = cancellation.clone().drop_guard();
        let root = self.root.clone();
        let variant = self.variant.clone();
        let control = RuntimePreparation::new(cancellation.clone(), None);
        let worker_control = control.clone();
        tokio::task::spawn_blocking(move || {
            verify_runtime_files_with_control(
                &root.join("assets"),
                &root.join("runtime"),
                &variant,
                &worker_control,
            )
        })
        .await
        .map_err(|error| PortError::Unexpected {
            message: error.to_string(),
        })?
        .map_err(|error| {
            if cancellation.is_cancelled() {
                PortError::Cancelled
            } else {
                PortError::Conflict {
                    resource: "installed translation runtime".into(),
                    message: error.to_string(),
                }
            }
        })?;
        let lease = self
            .runtime
            .acquire_with_control(intent, cancellation)
            .await?;
        cancel_on_drop.disarm();
        Ok(lease)
    }
}
