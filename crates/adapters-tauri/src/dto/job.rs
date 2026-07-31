use domain::chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressDto {
    pub percent: u8,
    pub message: String,
    pub current_step: Option<String>,
    pub processed_items: Option<u64>,
    pub total_items: Option<u64>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: String,
    pub revision: u64,
    pub project_id: Option<String>,
    pub title: String,
    pub status: String,
    pub stage: Option<String>,
    pub progress: JobProgressDto,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
