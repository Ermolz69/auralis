use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressDto {
    pub percent: u8,
    pub message: String,
    pub current_step: Option<String>,
    pub processed_items: Option<u64>,
    pub total_items: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobEventDto {
    pub kind: String,
    pub job_id: String,
    pub revision: u64,
    pub project_id: Option<String>,
    pub status: String,
    pub stage: Option<String>,
    pub progress: JobProgressDto,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdatedDto {
    pub project_id: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptReadyDto {
    pub project_id: String,
    pub job_id: String,
}
