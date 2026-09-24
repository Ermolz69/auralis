use crate::error::ApplicationError;
use uuid::Uuid;

pub struct TranslationReconcileItem {
    pub translation_id: Uuid,
    pub outcome: Result<(), ApplicationError>,
}
