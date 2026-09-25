use ports::translation_model_runtime::TranslationModelRuntimePort;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SelectedTranslationRuntime {
    current: RwLock<Option<Arc<dyn TranslationModelRuntimePort>>>,
}

impl SelectedTranslationRuntime {
    pub fn new(initial: Option<Arc<dyn TranslationModelRuntimePort>>) -> Self {
        Self {
            current: RwLock::new(initial),
        }
    }

    pub async fn read(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, Option<Arc<dyn TranslationModelRuntimePort>>> {
        self.current.read().await
    }

    pub async fn write(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, Option<Arc<dyn TranslationModelRuntimePort>>> {
        self.current.write().await
    }
}
