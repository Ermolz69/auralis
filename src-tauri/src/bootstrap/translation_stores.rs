use ports::translation_edit_intents::TranslationEditIntentStore;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use ports::translation_reviews::TranslationReviewStore;
use std::sync::Arc;

pub struct TranslationStores {
    pub links: Arc<dyn TranslationLinkStore>,
    pub jobs: Arc<dyn TranslationHostJobStore>,
    pub edits: Arc<dyn TranslationEditIntentStore>,
    pub reviews: Arc<dyn TranslationReviewStore>,
}
