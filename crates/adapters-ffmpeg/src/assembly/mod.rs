mod adapter;
mod config;
mod control;
mod error;
mod input;
mod pcm;
mod process;
mod slicing;
mod source;
mod staging;

pub use adapter::FfmpegAudioAssembler;
pub use config::AudioAssemblyLimits;
