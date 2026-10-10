use async_trait::async_trait;
use domain::project::ProjectId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::PortError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceCatalogEntry {
    pub voice_id: String,
    pub state: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub manifest_sha256: Option<String>,
    pub reference_count: Option<usize>,
    pub qualified_capabilities: Option<Vec<VoiceCapability>>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceCapability {
    pub engine_id: String,
    pub emotion_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceCatalogPage {
    pub items: Vec<VoiceCatalogEntry>,
    pub next_after: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceReferenceMetadata {
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub transcript: String,
    pub audio_sha256: String,
    pub qualification_status: String,
    pub qualification_report_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceDetails {
    pub voice_id: String,
    pub display_name: String,
    pub description: String,
    pub manifest_sha256: String,
    pub references: Vec<VoiceReferenceMetadata>,
    pub next_after_reference: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReferenceDetails {
    pub voice_id: String,
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub transcript: String,
    pub audio_sha256: String,
    pub audio_path: String,
    pub qualification_status: String,
    pub qualification_report_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceCompatibility {
    pub voice_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub compatible: bool,
    pub reference_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CastRole {
    pub display_name: String,
    pub voice_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogicalCast {
    pub version: u8,
    pub roles: BTreeMap<String, CastRole>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManualSelection {
    pub voice_id: String,
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CastSelectionSnapshot {
    pub voice_id: String,
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub manifest_sha256: String,
    pub audio_sha256: String,
    pub report_sha256: String,
    pub transcript: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CastRevision {
    pub version: u8,
    pub parent_revision: Option<String>,
    pub request_id: String,
    pub request_sha256: String,
    pub cast: LogicalCast,
    pub selections: BTreeMap<String, CastSelectionSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CastResult {
    pub revision_sha256: String,
    pub revision: CastRevision,
}

#[async_trait]
pub trait CasterPort: Send + Sync {
    async fn catalog_page(
        &self,
        project: &ProjectId,
        after: Option<String>,
        limit: u32,
    ) -> Result<VoiceCatalogPage, PortError>;
    async fn inspect_voice(
        &self,
        project: &ProjectId,
        voice_id: String,
        after_reference: Option<String>,
        limit: u32,
    ) -> Result<VoiceDetails, PortError>;
    async fn inspect_reference(
        &self,
        project: &ProjectId,
        voice_id: String,
        reference_id: String,
    ) -> Result<ReferenceDetails, PortError>;
    async fn check_compatibility(
        &self,
        project: &ProjectId,
        voice_id: String,
        engine_id: String,
        emotion_id: String,
    ) -> Result<VoiceCompatibility, PortError>;
    async fn current_cast(&self, project: &ProjectId) -> Result<Option<CastResult>, PortError>;
    async fn assign_roles(
        &self,
        project: &ProjectId,
        cast: LogicalCast,
        selections: BTreeMap<String, ManualSelection>,
        expected_revision: Option<String>,
        request_id: String,
    ) -> Result<CastResult, PortError>;
    async fn recover_cast(&self, project: &ProjectId) -> Result<String, PortError>;
}
