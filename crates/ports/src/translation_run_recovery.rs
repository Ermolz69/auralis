use async_trait::async_trait;
use domain::job::JobId;
use uuid::Uuid;

use crate::error::PortError;
use crate::translation_run_control::TranslationRunPhase;

#[async_trait]
pub trait TranslationRunRecoveryPort: Send + Sync {
    async fn recover_for_host(&self, run_id: Uuid, job_id: &JobId) -> Result<bool, PortError>;

    async fn phase(&self, run_id: Uuid) -> Result<TranslationRunPhase, PortError>;
}
