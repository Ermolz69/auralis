use domain::project::ProjectId;

pub struct CommitProjectDelete {
    pub project_id: ProjectId,
}

pub struct CommitProjectDeleteResult {
    pub deleted_job_ids: Vec<domain::job::JobId>,
}
