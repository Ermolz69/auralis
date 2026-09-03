#![allow(clippy::unwrap_used, clippy::expect_used)]
mod cleanup;
mod deletion;
mod path_guard;
mod resolver;
mod staging;
mod store;

#[cfg(test)]
mod extension_tests;
#[cfg(test)]
mod tests;

pub use store::LocalArtifactStore;
