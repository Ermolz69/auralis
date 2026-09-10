pub use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct CancelHandle {
    token: CancellationToken,
}

impl CancelHandle {
    pub fn new() -> (Self, CancellationToken) {
        let token = CancellationToken::new();
        (
            Self {
                token: token.clone(),
            },
            token,
        )
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

#[cfg(test)]
mod tests;
