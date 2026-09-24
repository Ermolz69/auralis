use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranslationSourceLanguageDto {
    Zh,
}

impl TranslationSourceLanguageDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zh => "zh",
        }
    }
}
