use super::{TranslationHistoryEntryDto, TranslationLinkDto};
use ports::translation_links::TranslationHistoryPage;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationHistoryPageDto {
    pub link: TranslationLinkDto,
    pub entries: Vec<TranslationHistoryEntryDto>,
    pub ready_count: u64,
    pub next_cursor: Option<String>,
}

impl From<TranslationHistoryPage> for TranslationHistoryPageDto {
    fn from(value: TranslationHistoryPage) -> Self {
        Self {
            link: value.link.into(),
            entries: value.entries.into_iter().map(Into::into).collect(),
            ready_count: value.ready_count,
            next_cursor: value.next_cursor.map(|id| id.to_string()),
        }
    }
}
