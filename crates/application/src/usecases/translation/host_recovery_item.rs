use crate::error::ApplicationError;
use domain::job::JobId;
use uuid::Uuid;

pub struct TranslationHostRecoveryItem {
    pub job_id: JobId,
    pub run_id: Uuid,
    pub outcome: Result<(), ApplicationError>,
}
