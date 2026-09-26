use crate::{
    committed_translation_progress::CommittedTranslationProgress,
    translation_start_guard::TranslationStartGuard,
};
use domain::job::JobId;
use tokio::sync::mpsc::UnboundedSender;

pub struct HostTranslationExecution {
    pub job_id: JobId,
    pub start_guard: Option<TranslationStartGuard>,
    pub progress: UnboundedSender<CommittedTranslationProgress>,
}
