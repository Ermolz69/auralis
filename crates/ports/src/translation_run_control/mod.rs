mod phase;
mod snapshot;

pub use phase::TranslationRunPhase;
pub use snapshot::TranslationRunSnapshot;

use crate::error::PortError;
use crate::translation_links::TranslationLink;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationRunControlPort: Send + Sync {
    async fn snapshot(&self, link: &TranslationLink) -> Result<TranslationRunSnapshot, PortError>;
    async fn request_pause(&self, link: &TranslationLink) -> Result<(), PortError>;
}
