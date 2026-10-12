use crate::{error::PortError, speech_workflow::SpeechSourceSnapshot};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerPreparation {
    pub version: String,
    pub original: SpeechSourceSnapshot,
    pub prepared: SpeechSourceSnapshot,
    pub changed_targets: u32,
    pub unresolved_targets: u32,
}

#[async_trait]
pub trait MarkerPort: Send + Sync {
    async fn prepare(&self, source: SpeechSourceSnapshot) -> Result<MarkerPreparation, PortError>;
}
