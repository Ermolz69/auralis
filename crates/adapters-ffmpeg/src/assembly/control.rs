use ports::{PortError, cancellation::CancellationToken};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct WorkControl {
    pub cancel: CancellationToken,
    pub deadline: Instant,
}

pub(super) use ports::cancellation::CancelOnDrop;

impl WorkControl {
    pub fn new(cancel: CancellationToken, timeout: Duration) -> Result<Self, PortError> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| super::error::invalid("Audio deadline overflow"))?;
        Ok(Self { cancel, deadline })
    }
    pub fn check(&self) -> Result<(), PortError> {
        if self.cancel.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(super::error::external(
                "Audio assembly total deadline exceeded",
            ));
        }
        Ok(())
    }
}
