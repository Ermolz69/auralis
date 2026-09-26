use domain::job::JobId;
use ports::translation_run_control::TranslationRunSnapshot;

#[derive(Clone, Debug)]
pub struct TranslationRunStatus {
    pub snapshot: TranslationRunSnapshot,
    pub active_host_job_id: Option<JobId>,
}
