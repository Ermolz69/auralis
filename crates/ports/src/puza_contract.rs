use async_trait::async_trait;
use domain::project::ProjectId;
use serde::{Deserialize, Serialize};

use crate::error::PortError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PuzaContractSnapshot {
    pub contract_version: String,
    pub project_json: String,
    pub cast_json: String,
    pub source_text: Option<String>,
    pub bindings_json: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PuzaVoiceBinding {
    pub role_id: String,
    pub display_name: String,
    pub voice_id: String,
    pub reference_id: String,
    pub engine_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_reference_permission: Option<String>,
}

#[async_trait]
pub trait PuzaContractPort: Send + Sync {
    async fn prepare_project(
        &self,
        project: &ProjectId,
        source_text: Option<String>,
        bindings: Vec<PuzaVoiceBinding>,
    ) -> Result<PuzaContractSnapshot, PortError>;
}
