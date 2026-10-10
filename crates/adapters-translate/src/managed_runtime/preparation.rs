use auralis_translation::ProviderError;
use auralis_translation_llamacpp::PreparationControl;
use ports::{cancellation::CancellationToken, error::PortError};
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct RuntimePreparation {
    cancellation: CancellationToken,
    deadline: Option<Instant>,
}

impl RuntimePreparation {
    pub(crate) fn new(cancellation: CancellationToken, deadline: Option<Instant>) -> Self {
        Self {
            cancellation,
            deadline,
        }
    }

    pub(crate) fn error(&self, cause: impl std::fmt::Display) -> PortError {
        if self.cancellation.is_cancelled() {
            PortError::Cancelled
        } else {
            PortError::ExternalToolFailed {
                tool: "managed model preparation".into(),
                message: cause.to_string(),
            }
        }
    }
}

impl PreparationControl for RuntimePreparation {
    fn check(&self) -> Result<(), ProviderError> {
        if self.cancellation.is_cancelled() {
            return Err(ProviderError::Permanent(
                "model preparation cancelled".into(),
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ProviderError::Permanent(
                "model preparation exceeded startup timeout".into(),
            ));
        }
        Ok(())
    }
}
