use super::AssemblySourceArtifact;
use domain::{
    project::ProjectId,
    speech::{Sha256Digest, timeline::AudioTimelineRequest},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioAssemblyPlan {
    pub project_id: ProjectId,
    pub source_run_id: Uuid,
    pub source_attempt_id: Uuid,
    pub raw_wav: AssemblySourceArtifact,
    pub raw_manifest: AssemblySourceArtifact,
    pub layout: AssemblySourceArtifact,
    pub ffmpeg_sha256: Sha256Digest,
    pub assembly_policy: String,
    pub timeline: AudioTimelineRequest,
}
