mod adapter;
mod command;
mod containment;
mod dto;
mod error;
mod parser;
mod validation;

#[cfg(test)]
mod tests;

pub use adapter::YtDlpAdapter;
pub use error::YtDlpError;
