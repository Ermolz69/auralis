use ports::translation_inspection::TranslationSourceFormat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationFormatDto {
    Srt,
    Vtt,
}

impl From<TranslationFormatDto> for TranslationSourceFormat {
    fn from(value: TranslationFormatDto) -> Self {
        match value {
            TranslationFormatDto::Srt => Self::Srt,
            TranslationFormatDto::Vtt => Self::Vtt,
        }
    }
}

impl From<TranslationSourceFormat> for TranslationFormatDto {
    fn from(value: TranslationSourceFormat) -> Self {
        match value {
            TranslationSourceFormat::Srt => Self::Srt,
            TranslationSourceFormat::Vtt => Self::Vtt,
        }
    }
}
