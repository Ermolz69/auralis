use domain::job::Job;
use domain::project::ProjectId;

pub struct CommitJobUpdate {
    pub job: Job,
    pub expected_revision: u64,
}

pub struct CommitTerminalJobUpdate {
    pub job: Job,
    pub expected_revision: u64,
    pub deduplication_key: String,
    pub project_id: ProjectId,
    pub outcome: domain::job::TerminalOutcome,
}

pub struct ApplyTerminalLifecycle {
    pub project_id: ProjectId,
    pub job_id: domain::job::JobId,
    pub outcome: domain::job::TerminalOutcome,
}
