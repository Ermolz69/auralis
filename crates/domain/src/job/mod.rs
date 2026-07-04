#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JobId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: JobId,
    pub project_id: crate::project::ProjectId,
    pub status: JobStatus,
}
