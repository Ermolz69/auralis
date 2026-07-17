use crate::dto::job::JobDto;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobLifecycleEventKindDto {
    Created,
    Started,
    Progressed,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobEventDto {
    pub kind: JobLifecycleEventKindDto,
    pub job: JobDto,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUpdatedDto {
    pub project_id: String,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptReadyDto {
    pub project_id: String,
    pub job_id: String,
}
