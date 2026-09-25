use super::TranslationPublicationState;
use domain::media::ArtifactId;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationPublicationRecord {
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub result_revision: u32,
    pub run_id: Uuid,
    pub artifact_id: ArtifactId,
    pub source_sha256: String,
    pub expected_link_revision: u64,
    pub needs_review: bool,
    pub state: TranslationPublicationState,
}
