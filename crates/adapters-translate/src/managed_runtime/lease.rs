use async_trait::async_trait;
use common::model_admission::ModelAdmissionLease;
use common::owned_process::OwnedChild;
use ports::error::PortError;
use ports::translation_model_runtime::TranslationModelLease;
use std::io;

pub struct ManagedLlamaLease {
    server_url: String,
    child: Option<OwnedChild>,
    admission: Option<ModelAdmissionLease>,
}

impl ManagedLlamaLease {
    pub fn new(server_url: String, child: OwnedChild, admission: ModelAdmissionLease) -> Self {
        Self {
            server_url,
            child: Some(child),
            admission: Some(admission),
        }
    }

    pub fn exited(&mut self) -> io::Result<bool> {
        self.child
            .as_mut()
            .ok_or_else(|| io::Error::other("model child was already stopped"))?
            .exited()
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        if let Some(mut child) = self.child.take() {
            if let Err(error) = child.stop().await {
                if let Some(admission) = self.admission.take() {
                    std::mem::forget(admission);
                }
                return Err(error);
            }
        }
        if let Some(admission) = self.admission.take() {
            admission.release().await?;
        }
        Ok(())
    }
}

impl Drop for ManagedLlamaLease {
    fn drop(&mut self) {
        let (Some(mut child), Some(admission)) = (self.child.take(), self.admission.take()) else {
            return;
        };
        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                std::mem::forget(admission);
                drop(child);
                return;
            };
            if runtime.block_on(child.stop()).is_ok() {
                let _ = runtime.block_on(admission.release());
            } else {
                std::mem::forget(admission);
            }
        });
    }
}

#[async_trait]
impl TranslationModelLease for ManagedLlamaLease {
    fn server_url(&self) -> &str {
        &self.server_url
    }

    async fn stop(mut self: Box<Self>) -> Result<(), PortError> {
        ManagedLlamaLease::stop(&mut self)
            .await
            .map_err(|error| PortError::CleanupFailed {
                resource: "managed model process tree".into(),
                message: error.to_string(),
            })
    }
}
