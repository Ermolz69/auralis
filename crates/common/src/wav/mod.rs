mod entity;
mod error;
mod inspect;
#[cfg(test)]
mod tests;

pub use entity::WavProperties;
pub use error::Pcm16Error;
pub use inspect::{inspect_pcm16, inspect_pcm16_wav};
