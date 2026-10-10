use async_trait::async_trait;
use domain::speech::{Sha256Digest, SpeechId, SpeechPlan, SynthesisResult};

use crate::cancellation::CancellationToken;
use crate::error::PortError;

#[derive(Debug, Clone)]
pub struct SpeechExecutionIntent {
    pub project_id: SpeechId,
    pub run_id: SpeechId,
    pub attempt_id: SpeechId,
    pub idempotency_key: SpeechId,
    pub plan_sha256: Sha256Digest,
    pub model_sha256: Sha256Digest,
    pub profile_sha256: Sha256Digest,
}

#[async_trait]
pub trait SpeechExecutionPort: Send + Sync {
    async fn execute(
        &self,
        plan: &SpeechPlan,
        intent: &SpeechExecutionIntent,
        cancellation: CancellationToken,
    ) -> Result<SynthesisResult, PortError>;
}
