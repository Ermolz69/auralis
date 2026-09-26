mod compared_segment;
mod comparison_page;
mod edit_provenance;
mod reader;
mod verified;

pub use compared_segment::ComparedTranslationSegment;
pub use comparison_page::{MAX_COMPARISON_PAGE_SIZE, TranslationComparisonPage};
pub use edit_provenance::TranslationEditProvenance;
pub use reader::TranslationResultPort;
pub use verified::VerifiedTranslationResult;
