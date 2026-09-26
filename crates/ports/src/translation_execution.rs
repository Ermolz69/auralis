use crate::error::PortError;
use crate::host_translation_execution::HostTranslationExecution;
use crate::translation_links::{TranslationLink, TranslationRunIntent};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

#[async_trait]
pub trait TranslationExecutionPort: Send + Sync {
    async fn execute(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        server_url: &str,
    ) -> Result<Uuid, PortError>;

    async fn execute_for_host(
        &self,
        _source_path: &Path,
        _link: &TranslationLink,
        _intent: &TranslationRunIntent,
        _server_url: &str,
        _host: HostTranslationExecution,
    ) -> Result<Uuid, PortError> {
        Err(PortError::Unsupported {
            message: "host-owned translation execution is unavailable".into(),
        })
    }
}
