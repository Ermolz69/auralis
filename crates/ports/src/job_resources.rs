use serde::{Deserialize, Serialize};

/// CPU units include threads used by child processes, not just Rust workers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDemand {
    pub cpu: u32,
    pub network: u32,
    pub disk: u32,
    pub processes: u32,
}

impl ResourceDemand {
    pub fn fits(self, available: Self) -> bool {
        self.cpu <= available.cpu
            && self.network <= available.network
            && self.disk <= available.disk
            && self.processes <= available.processes
    }

    pub fn overlaps(self, other: Self) -> bool {
        (self.cpu > 0 && other.cpu > 0)
            || (self.network > 0 && other.network > 0)
            || (self.disk > 0 && other.disk > 0)
            || (self.processes > 0 && other.processes > 0)
    }
}

#[async_trait::async_trait]
pub trait CpuExecutorPort: Send + Sync {
    /// Returns only after the closure stops. Implementations bound both workers and backlog.
    async fn run(&self, work: Box<dyn FnOnce() + Send + 'static>) -> Result<(), crate::PortError>;
}
