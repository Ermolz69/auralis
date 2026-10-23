use crate::error::PortError;
use async_trait::async_trait;
use domain::library::LibraryState;

#[async_trait]
pub trait LibraryRepository: Send + Sync {
    async fn load(&self) -> Result<LibraryState, PortError>;
    async fn save(&self, state: LibraryState) -> Result<LibraryState, PortError>;
}

#[async_trait]
pub trait LibraryTextReader: Send + Sync {
    async fn read_text(&self, path: &str) -> Result<String, PortError>;
}
