use domain::job::JobSnapshot;

use crate::job_scheduler::ScheduledJob;

impl From<JobSnapshot> for ScheduledJob {
    fn from(snapshot: JobSnapshot) -> Self {
        Self {
            id: snapshot.id,
            kind: snapshot.kind,
            revision: snapshot.revision,
            project_id: Some(snapshot.project_id),
            title: snapshot.title,
            status: snapshot.status,
            stage: snapshot.stage,
            progress: snapshot.progress,
            error: snapshot.error.map(|error| error.message),
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
        }
    }
}
