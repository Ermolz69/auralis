use ports::{
    error::PortError,
    translation_links::TranslationLink,
    translation_run_control::{TranslationRunControlPort, TranslationRunSnapshot},
    translation_start_guard::TranslationStartGuard,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

pub struct CompletionTranslationControl {
    inner: Arc<dyn TranslationRunControlPort>,
    completed: Arc<AtomicBool>,
    checked: Arc<AtomicBool>,
    fail_control: bool,
}

impl CompletionTranslationControl {
    pub fn new(
        inner: Arc<dyn TranslationRunControlPort>,
        completed: Arc<AtomicBool>,
        checked: Arc<AtomicBool>,
        fail_control: bool,
    ) -> Self {
        Self {
            inner,
            completed,
            checked,
            fail_control,
        }
    }
}

#[async_trait::async_trait]
impl TranslationRunControlPort for CompletionTranslationControl {
    async fn capture_start(
        &self,
        link: &TranslationLink,
    ) -> Result<TranslationStartGuard, PortError> {
        if self.completed.load(Ordering::SeqCst) && !self.checked.swap(true, Ordering::SeqCst) {
            self.inner.request_pause(link).await?;
            if self.fail_control {
                return Err(PortError::Storage {
                    operation: "completion control read",
                    message: "injected final guard read failure".into(),
                });
            }
        }
        self.inner.capture_start(link).await
    }

    async fn snapshot(&self, link: &TranslationLink) -> Result<TranslationRunSnapshot, PortError> {
        self.inner.snapshot(link).await
    }

    async fn snapshot_for_run(
        &self,
        link: &TranslationLink,
        run_id: Uuid,
    ) -> Result<TranslationRunSnapshot, PortError> {
        self.inner.snapshot_for_run(link, run_id).await
    }

    async fn request_pause(&self, link: &TranslationLink) -> Result<(), PortError> {
        self.inner.request_pause(link).await
    }
}
