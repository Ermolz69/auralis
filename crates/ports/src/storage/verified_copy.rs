use domain::{media::ArtifactKind, project::ProjectId, speech::Sha256Digest};
use std::path::PathBuf;

pub struct VerifiedArtifactCopy {
    pub project_id: ProjectId,
    pub kind: ArtifactKind,
    pub source_path: PathBuf,
    pub filename_hint: Option<String>,
    pub expected_sha256: Sha256Digest,
    pub max_bytes: u64,
}
