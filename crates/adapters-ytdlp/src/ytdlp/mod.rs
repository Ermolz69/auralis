mod adapter;
mod command;
mod subtitle_process;
pub use owned_process::cancellable::ProcessLimits;
mod containment;
mod dto;
mod error;
mod owned_process;
mod parser;
mod validation;

#[cfg(test)]
mod tests;

pub use adapter::YtDlpAdapter;
pub use error::YtDlpError;
