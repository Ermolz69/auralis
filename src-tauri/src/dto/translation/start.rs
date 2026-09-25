use domain::job::Job;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationStartDto {
    pub job_id: String,
    pub run_id: String,
    pub state: &'static str,
}

impl TranslationStartDto {
    pub fn accepted(job: Job, run_id: Uuid) -> Self {
        Self {
            job_id: job.id().to_string(),
            run_id: run_id.to_string(),
            state: "accepted",
        }
    }
}
