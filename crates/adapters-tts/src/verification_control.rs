use ports::{PortError, cancellation::CancellationToken};

pub(crate) struct VerificationGuard(pub CancellationToken);

impl Drop for VerificationGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(crate) fn check_cancel(cancel: &CancellationToken) -> Result<(), PortError> {
    if cancel.is_cancelled() {
        Err(PortError::Cancelled)
    } else {
        Ok(())
    }
}
