#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
pub(super) use unix::{OwnedChild, spawn};
#[cfg(windows)]
pub(super) use windows::{OwnedChild, spawn};
pub(super) mod cancellable;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod tests;
