use super::owned_process::OwnedChild;
use ports::translation_model_runtime::TranslationModelLease;
use std::io;
use tokio::sync::OwnedSemaphorePermit;

pub struct ManagedLlamaLease {
    server_url: String,
    child: OwnedChild,
    _permit: OwnedSemaphorePermit,
    _admission: Option<common::model_admission::ModelAdmissionLease>,
}

impl ManagedLlamaLease {
    pub fn new(
        server_url: String,
        child: OwnedChild,
        permit: OwnedSemaphorePermit,
        admission: common::model_admission::ModelAdmissionLease,
    ) -> Self {
        Self {
            server_url,
            child,
            _permit: permit,
            _admission: Some(admission),
        }
    }

    pub fn exited(&mut self) -> io::Result<bool> {
        self.child.exited()
    }

    pub async fn stop(&mut self) -> io::Result<()> {
        let result = self.child.stop().await;
        if result.is_err()
            && let Some(admission) = self._admission.take()
        {
            std::mem::forget(admission);
        }
        result
    }
}

impl TranslationModelLease for ManagedLlamaLease {
    fn server_url(&self) -> &str {
        &self.server_url
    }
}
