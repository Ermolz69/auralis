use async_trait::async_trait;
use domain::speech::{SpeechPlan, SynthesisResult, VoiceSnapshot};

use crate::error::PortError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechContractIssue {
    pub code: String,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechContractReport {
    pub scope: String,
    pub valid: bool,
    pub errors: Vec<SpeechContractIssue>,
}

#[async_trait]
pub trait SpeechContractPort: Send + Sync {
    async fn validate_plan(&self, plan: &SpeechPlan) -> Result<SpeechContractReport, PortError>;

    async fn validate_voice(
        &self,
        snapshot: &VoiceSnapshot,
    ) -> Result<SpeechContractReport, PortError>;

    async fn validate_result(
        &self,
        plan: &SpeechPlan,
        result: &SynthesisResult,
    ) -> Result<SpeechContractReport, PortError>;
}
