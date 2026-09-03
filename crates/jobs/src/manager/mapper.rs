use domain::job::Job;
use ports::job_scheduler::ScheduledJob;

pub fn map_job_to_scheduled(job: &Job) -> ScheduledJob {
    ScheduledJob {
        id: job.id().clone(),
        kind: job.kind().clone(),
        revision: job.revision(),
        project_id: Some(job.project_id().clone()),
        title: job.title().to_string(),
        status: job.status().clone(),
        stage: job.stage().cloned(),
        progress: job.progress().clone(),
        error: job.error().map(|e| e.message.clone()),
        created_at: *job.created_at(),
        updated_at: *job.updated_at(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{job::JobKind, project::ProjectId};

    #[test]
    fn scheduled_job_preserves_domain_kind() {
        let job = Job::new(ProjectId::new(), "Subtitles".into(), JobKind::Dubbing);
        assert_eq!(map_job_to_scheduled(&job).kind, *job.kind());
    }
}
