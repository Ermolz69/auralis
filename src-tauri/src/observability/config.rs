use std::path::PathBuf;
use tracing_appender::rolling::Rotation;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogFormat {
    Pretty,
    Compact,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OverflowPolicy {
    DropWhenFull,
    Backpressure,
}

#[derive(Clone, Debug)]
pub enum LogDestinationErrorKind {
    PathResolutionFailed,
    UnsupportedPlatform,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum LogDestination {
    Directory(PathBuf),
    Disabled,
    Unavailable(LogDestinationErrorKind),
}

use tracing_subscriber::EnvFilter;

pub struct ObservabilityConfig {
    pub default_filter: EnvFilter,
    pub console_format: LogFormat,
    pub file_format: LogFormat,
    pub log_dir: LogDestination,
    pub rotation: Rotation,
    pub max_log_files: usize,
    pub buffer_capacity: usize,
    pub overflow_policy: OverflowPolicy,
}

impl ObservabilityConfig {
    pub fn for_build(log_dir: LogDestination, is_debug: bool) -> Self {
        if is_debug {
            Self {
                default_filter: EnvFilter::new("debug"),
                console_format: LogFormat::Pretty,
                file_format: LogFormat::Json,
                log_dir,
                rotation: Rotation::DAILY,
                max_log_files: 30,
                buffer_capacity: 8192,
                overflow_policy: OverflowPolicy::DropWhenFull,
            }
        } else {
            Self {
                default_filter: EnvFilter::new("info"),
                console_format: LogFormat::Compact,
                file_format: LogFormat::Json,
                log_dir,
                rotation: Rotation::DAILY,
                max_log_files: 30,
                buffer_capacity: 8192,
                overflow_policy: OverflowPolicy::DropWhenFull,
            }
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_log_files == 0 {
            return Err("max_log_files must be greater than 0");
        }
        if self.buffer_capacity == 0 {
            return Err("buffer_capacity must be greater than 0");
        }
        Ok(())
    }
}
