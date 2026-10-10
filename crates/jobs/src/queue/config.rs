use ports::{PortError, job_resources::ResourceDemand};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct JobQueueConfig {
    pub budgets: ResourceDemand,
    pub pending_capacity: usize,
    pub dispatch_window: usize,
    pub max_active: usize,
    pub max_bypasses: usize,
    pub cpu_backlog: usize,
    pub attach_timeout: Duration,
    pub storage_retry_interval: Duration,
    pub cleanup_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub max_subtitle_bytes: u64,
}

impl Default for JobQueueConfig {
    fn default() -> Self {
        let cpu = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1).clamp(1, 4));
        Self {
            budgets: ResourceDemand {
                cpu: cpu as u32,
                network: 2,
                disk: 2,
                processes: 2,
            },
            pending_capacity: 1024,
            dispatch_window: 64,
            max_active: 64,
            max_bypasses: 8,
            cpu_backlog: 64,
            attach_timeout: Duration::from_secs(5),
            storage_retry_interval: Duration::from_millis(500),
            cleanup_timeout: Duration::from_secs(7),
            shutdown_timeout: Duration::from_secs(10),
            max_subtitle_bytes: 10 * 1024 * 1024,
        }
    }
}

impl JobQueueConfig {
    pub fn validate(&self) -> Result<(), PortError> {
        if [
            self.budgets.cpu,
            self.budgets.network,
            self.budgets.disk,
            self.budgets.processes,
        ]
        .iter()
        .any(|n| *n == 0 || *n > 256)
            || self.pending_capacity == 0
            || self.pending_capacity > 100_000
            || self.dispatch_window == 0
            || self.dispatch_window > self.pending_capacity
            || self.max_active == 0
            || self.max_active > 256
            || self.max_bypasses == 0
            || self.max_bypasses > 10_000
            || self.cpu_backlog == 0
            || self.cpu_backlog > 1024
            || self.max_subtitle_bytes == 0
            || self.max_subtitle_bytes > 100 * 1024 * 1024
            || [
                self.attach_timeout,
                self.storage_retry_interval,
                self.cleanup_timeout,
                self.shutdown_timeout,
            ]
            .iter()
            .any(|v| v.is_zero() || *v > Duration::from_secs(300))
        {
            return Err(PortError::InvalidRuntimeConfiguration {
                message: "Resource, queue or timeout bounds exceeded".into(),
            });
        }
        Ok(())
    }

    pub fn validate_demand(&self, demand: ResourceDemand) -> Result<(), PortError> {
        if demand == ResourceDemand::default()
            || !demand.fits(self.budgets)
            || (demand.processes > 0 && demand.cpu == 0)
        {
            return Err(PortError::InvalidResourceRequest {
                message: format!("{demand:?} exceeds {:?} or is empty", self.budgets),
            });
        }
        Ok(())
    }
}
