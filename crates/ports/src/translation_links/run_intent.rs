use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationRunIntent {
    pub run_id: Uuid,
    pub translation_id: Uuid,
    pub profile_manifest_json: String,
    pub profile_sha256: String,
    pub max_target_segments: u32,
    pub context_before_segments: u32,
    pub context_after_segments: u32,
    pub parser_version: u32,
    pub policy_fingerprint: String,
    pub created_at: DateTime<Utc>,
}
