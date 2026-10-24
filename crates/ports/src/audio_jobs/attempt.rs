use domain::job::Job;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ActiveAssemblyAttempt {
    pub assembly_id: Uuid,
    pub attempt_id: Uuid,
    pub job: Job,
}
