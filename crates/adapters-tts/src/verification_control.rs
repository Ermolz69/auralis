use ports::{PortError, cancellation::CancellationToken};

pub(crate) use ports::cancellation::CancelOnDrop as VerificationGuard;

pub(crate) fn check_cancel(cancel: &CancellationToken) -> Result<(), PortError> {
    if cancel.is_cancelled() {
        Err(PortError::Cancelled)
    } else {
        Ok(())
    }
}
