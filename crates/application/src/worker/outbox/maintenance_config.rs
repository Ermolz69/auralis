use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OutboxMaintenanceConfig {
    pub interval: Duration,
    pub staging_max_age: Duration,
    pub workspace_max_age: Duration,
    pub done_retention: domain::chrono::TimeDelta,
    pub dead_retention: domain::chrono::TimeDelta,
    pub per_status_batch_limit: u32,
    pub max_batches: u32,
    pub run_on_startup: bool,
    pub max_active: usize,
    pub fetch_batch: usize,
    pub lease_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub shutdown_timeout: Duration,
}

impl OutboxMaintenanceConfig {
    pub fn try_default() -> Result<Self, crate::error::ApplicationError> {
        Ok(Self {
            interval: Duration::from_secs(3600),
            staging_max_age: Duration::from_secs(86400),
            workspace_max_age: Duration::from_secs(86400),
            done_retention: domain::chrono::TimeDelta::try_days(7).ok_or_else(|| {
                crate::error::ApplicationError::Configuration("Invalid done_retention".into())
            })?,
            dead_retention: domain::chrono::TimeDelta::try_days(30).ok_or_else(|| {
                crate::error::ApplicationError::Configuration("Invalid dead_retention".into())
            })?,
            per_status_batch_limit: 500,
            max_batches: 10,
            run_on_startup: true,
            max_active: 2,
            fetch_batch: 32,
            lease_timeout: Duration::from_secs(300),
            heartbeat_interval: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(30),
        })
    }

    pub fn validate(&self) -> Result<(), crate::error::ApplicationError> {
        if self.interval.is_zero() {
            return Err(crate::error::ApplicationError::Configuration(
                "interval must be non-zero".to_string(),
            ));
        }
        if self.staging_max_age.is_zero() {
            return Err(crate::error::ApplicationError::Configuration(
                "staging_max_age must be non-zero".to_string(),
            ));
        }
        if self.workspace_max_age.is_zero() {
            return Err(crate::error::ApplicationError::Configuration(
                "workspace_max_age must be non-zero".to_string(),
            ));
        }
        if self.done_retention.num_seconds() <= 0 {
            return Err(crate::error::ApplicationError::Configuration(
                "done_retention must be strictly positive".to_string(),
            ));
        }
        if self.dead_retention.num_seconds() <= 0 {
            return Err(crate::error::ApplicationError::Configuration(
                "dead_retention must be strictly positive".to_string(),
            ));
        }
        if self.dead_retention < self.done_retention {
            return Err(crate::error::ApplicationError::Configuration(
                "dead_retention must be >= done_retention".to_string(),
            ));
        }
        if self.per_status_batch_limit == 0 {
            return Err(crate::error::ApplicationError::Configuration(
                "per_status_batch_limit must be non-zero".to_string(),
            ));
        }
        if self.max_batches == 0 {
            return Err(crate::error::ApplicationError::Configuration(
                "max_batches must be non-zero".to_string(),
            ));
        }
        if self.shutdown_timeout.is_zero() {
            return Err(crate::error::ApplicationError::Configuration(
                "shutdown_timeout must be non-zero".to_string(),
            ));
        }
        if self.max_active == 0
            || self.max_active > 16
            || self.fetch_batch == 0
            || self.fetch_batch > 1024
            || self.fetch_batch < self.max_active
            || self.heartbeat_interval.is_zero()
            || self.lease_timeout <= self.heartbeat_interval * 2
            || self.lease_timeout > Duration::from_secs(3600)
        {
            return Err(crate::error::ApplicationError::Configuration(
                "Invalid outbox concurrency or lease bounds".to_string(),
            ));
        }
        Ok(())
    }
}
