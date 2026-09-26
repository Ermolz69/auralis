use ports::{
    cancellation::CancellationToken,
    error::PortError,
    translation_links::TranslationRunIntent,
    translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct CompletingTranslationRuntime {
    pub completed: Arc<AtomicBool>,
    cleanup_fails: bool,
}

impl CompletingTranslationRuntime {
    pub fn new(cleanup_fails: bool) -> Arc<Self> {
        Arc::new(Self {
            completed: Arc::new(AtomicBool::new(false)),
            cleanup_fails,
        })
    }
}

#[async_trait::async_trait]
impl TranslationModelRuntimePort for CompletingTranslationRuntime {
    async fn acquire_with_control(
        &self,
        _: &TranslationRunIntent,
        _: CancellationToken,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        self.completed.store(true, Ordering::SeqCst);
        Err(if self.cleanup_fails {
            PortError::CleanupFailed {
                resource: "completion test runtime".into(),
                message: "cleanup failed at acquisition completion".into(),
            }
        } else {
            PortError::ExternalToolFailed {
                tool: "completion test runtime".into(),
                message: "ordinary acquisition failure".into(),
            }
        })
    }
}
