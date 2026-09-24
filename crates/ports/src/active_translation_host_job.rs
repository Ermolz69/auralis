use domain::job::Job;
use uuid::Uuid;

pub struct ActiveTranslationHostJob {
    pub job: Job,
    pub run_id: Uuid,
}
