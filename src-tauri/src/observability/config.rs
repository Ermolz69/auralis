use super::error::ObservabilityValidationError;
use std::path::PathBuf;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::EnvFilter;

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

#[derive(Clone, Debug)]
pub struct ObservabilitySettings {
    pub default_filter: String,
    pub console_format: LogFormat,
    pub file_format: LogFormat,
    pub rotation: Rotation,
    pub max_log_files: usize,
    pub buffer_capacity: usize,
    pub overflow_policy: OverflowPolicy,
}

pub struct ValidatedObservabilitySettings {
    pub default_filter: EnvFilter,
    pub console_format: LogFormat,
    pub file_format: LogFormat,
    pub rotation: Rotation,
    pub max_log_files: usize,
    pub buffer_capacity: usize,
    pub overflow_policy: OverflowPolicy,
}

impl TryFrom<ObservabilitySettings> for ValidatedObservabilitySettings {
    type Error = ObservabilityValidationError;

    fn try_from(settings: ObservabilitySettings) -> Result<Self, Self::Error> {
        if settings.max_log_files == 0 {
            return Err(ObservabilityValidationError::ZeroMaxLogFiles);
        }
        if settings.buffer_capacity == 0 {
            return Err(ObservabilityValidationError::ZeroBufferCapacity);
        }
        let filter = EnvFilter::try_new(&settings.default_filter)
            .map_err(|_| ObservabilityValidationError::InvalidDefaultFilter)?;

        Ok(Self {
            default_filter: filter,
            console_format: settings.console_format,
            file_format: settings.file_format,
            rotation: settings.rotation,
            max_log_files: settings.max_log_files,
            buffer_capacity: settings.buffer_capacity,
            overflow_policy: settings.overflow_policy,
        })
    }
}

impl ObservabilitySettings {
    pub fn for_build(is_debug: bool) -> Self {
        if is_debug {
            Self {
                default_filter: "debug".to_string(),
                console_format: LogFormat::Pretty,
                file_format: LogFormat::Json,
                rotation: Rotation::DAILY,
                max_log_files: 30,
                buffer_capacity: 8192,
                overflow_policy: OverflowPolicy::DropWhenFull,
            }
        } else {
            Self {
                default_filter: "info".to_string(),
                console_format: LogFormat::Compact,
                file_format: LogFormat::Json,
                rotation: Rotation::DAILY,
                max_log_files: 30,
                buffer_capacity: 8192,
                overflow_policy: OverflowPolicy::DropWhenFull,
            }
        }
    }
}

pub struct ObservabilityConfig {
    pub settings: ValidatedObservabilitySettings,
    pub log_dir: LogDestination,
}
