use super::{TranslationLink, TranslationPublicationRecord};
use uuid::Uuid;

pub const MAX_TRANSLATION_HISTORY_PAGE_SIZE: u32 = 100;

pub struct TranslationHistoryPage {
    pub link: TranslationLink,
    pub entries: Vec<TranslationPublicationRecord>,
    pub next_cursor: Option<Uuid>,
}
