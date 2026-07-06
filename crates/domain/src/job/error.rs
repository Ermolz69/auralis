#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl JobError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}
