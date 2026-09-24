use async_trait::async_trait;
use domain::job::JobId;
use uuid::Uuid;

use crate::error::PortError;
use crate::recovered_translation_run::RecoveredTranslationRun;

#[async_trait]
pub trait TranslationRunRecoveryPort: Send + Sync {
    async fn recover_for_host(&self, run_id: Uuid, job_id: &JobId) -> Result<bool, PortError>;

    async fn state(&self, run_id: Uuid) -> Result<RecoveredTranslationRun, PortError>;
}
