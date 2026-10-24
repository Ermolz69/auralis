use crate::error::PortError;
use async_trait::async_trait;
use domain::library::LibraryState;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaOperation {
    pub id: String,
    pub project_id: String,
    pub asset_id: String,
    pub status: String,
    pub completed_bytes: u64,
    pub total_bytes: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMaterialContext {
    pub project_id: String,
    pub asset_id: String,
    pub version: Option<String>,
    pub artifact_id: Option<String>,
    pub track_id: Option<String>,
}

#[async_trait]
pub trait MediaContractValidator: Send + Sync {
    async fn validate(&self, root: &Path) -> Result<(), PortError>;
}

#[async_trait]
pub trait MediaProjectStore: Send + Sync {
    async fn ensure(&self, project_id: &str) -> Result<(), PortError>;
    async fn bind_run(
        &self,
        project_id: &str,
        asset_id: &str,
        run_id: &str,
    ) -> Result<(), PortError>;
    async fn reconcile(&self, state: LibraryState) -> Result<LibraryState, PortError>;
    async fn attach(
        &self,
        operation_id: &str,
        project_id: &str,
        asset_id: &str,
        duplicate: bool,
    ) -> Result<LibraryState, PortError>;
    async fn operation(&self, id: &str) -> Result<Option<MediaOperation>, PortError>;
    async fn cancel(&self, id: &str) -> Result<(), PortError>;
    async fn structure(&self, project_id: &str) -> Result<serde_json::Value, PortError>;
    async fn validate(&self, project_id: &str) -> Result<(), PortError>;
}
