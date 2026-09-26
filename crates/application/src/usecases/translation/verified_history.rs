use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_result::VerifiedTranslationResult;
use std::path::PathBuf;

pub struct VerifiedTranslationHistory {
    pub link: TranslationLink,
    pub intent: TranslationRunIntent,
    pub source_path: PathBuf,
    pub result: VerifiedTranslationResult,
}
