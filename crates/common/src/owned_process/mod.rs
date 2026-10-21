#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{OwnedChild, spawn};
#[cfg(windows)]
pub use windows::{OwnedChild, spawn};
