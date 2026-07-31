#![allow(clippy::unwrap_used, clippy::expect_used)]
mod usecase;

pub use usecase::*;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
