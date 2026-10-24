use domain::{project::ProjectId, speech::Sha256Digest};
use uuid::Uuid;

pub const MAX_ASSEMBLY_PLAN_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAssemblyIntent {
    pub assembly_id: Uuid,
    pub project_id: ProjectId,
    pub source_run_id: Uuid,
    pub source_attempt_id: Uuid,
    pub plan_json: String,
    pub plan_sha256: Sha256Digest,
}
