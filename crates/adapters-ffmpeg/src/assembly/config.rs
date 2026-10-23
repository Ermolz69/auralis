use ports::PortError;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AudioAssemblyLimits {
    pub max_input_bytes: u64,
    pub max_pcm_bytes: u64,
    pub process_timeout: Duration,
    pub total_timeout: Duration,
    pub max_stderr_bytes: usize,
}

impl Default for AudioAssemblyLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1_u64 << 32,
            max_pcm_bytes: (1_u64 << 32) - 128,
            process_timeout: Duration::from_secs(600),
            total_timeout: Duration::from_secs(1800),
            max_stderr_bytes: 65_536,
        }
    }
}

impl AudioAssemblyLimits {
    pub(super) fn validate(&self) -> Result<(), PortError> {
        if self.max_input_bytes == 0
            || self.max_input_bytes > 1_u64 << 32
            || self.max_pcm_bytes == 0
            || self.max_pcm_bytes > (1_u64 << 32) - 128
            || self.process_timeout.is_zero()
            || self.process_timeout > Duration::from_secs(600)
            || self.total_timeout < self.process_timeout
            || self.total_timeout > Duration::from_secs(1800)
            || self.max_stderr_bytes == 0
            || self.max_stderr_bytes > 65_536
        {
            return Err(super::error::invalid(
                "Invalid bounded audio assembly limits",
            ));
        }
        Ok(())
    }
}
