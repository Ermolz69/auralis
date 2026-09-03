#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
pub(super) use unix::spawn;
#[cfg(windows)]
pub(super) use windows::spawn;
#[cfg(test)]
mod tests;
