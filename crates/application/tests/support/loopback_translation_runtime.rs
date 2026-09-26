use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};

pub struct LoopbackTranslationRuntime(pub String);

impl TranslationModelLease for LoopbackTranslationRuntime {
    fn server_url(&self) -> &str {
        &self.0
    }
}

#[async_trait::async_trait]
impl TranslationModelRuntimePort for LoopbackTranslationRuntime {
    async fn acquire(
        &self,
        _: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        Ok(Box::new(Self(self.0.clone())))
    }
}
