use crate::error::PortError;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationModelLease: Send {
    fn server_url(&self) -> &str;

    async fn stop(self: Box<Self>) -> Result<(), PortError> {
        Ok(())
    }
}
