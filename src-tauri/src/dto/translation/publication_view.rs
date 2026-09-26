use super::{TranslationLinkDto, TranslationPublicationDto};
use ports::translation_links::TranslationPublicationView;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationPublicationViewDto {
    pub link: TranslationLinkDto,
    pub publication: TranslationPublicationDto,
}
impl From<TranslationPublicationView> for TranslationPublicationViewDto {
    fn from(value: TranslationPublicationView) -> Self {
        Self {
            link: value.link.into(),
            publication: value.publication.into(),
        }
    }
}
