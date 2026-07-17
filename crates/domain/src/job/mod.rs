#![allow(clippy::unwrap_used, clippy::expect_used)]
mod entity;
mod error;
mod events;
mod id;
mod kind;
mod progress;
mod snapshot;
mod status;

pub use entity::Job;
pub use error::JobError;
pub use events::JobEvent;
pub use id::JobId;
pub use kind::JobKind;
pub use progress::JobProgress;
pub use snapshot::JobSnapshot;
pub use status::{JobStatus, TerminalOutcome};

pub const MAX_JOB_REVISION: u64 = 9_007_199_254_740_991;

#[cfg(test)]
mod tests;
