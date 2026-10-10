use ports::job_resources::ResourceDemand;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

pub(crate) struct ResourcePool {
    available: Mutex<(ResourceDemand, usize)>,
    max_active: usize,
    pub wake: Arc<Notify>,
}

pub(crate) struct ResourceLease {
    pool: Arc<ResourcePool>,
    demand: ResourceDemand,
}

impl ResourcePool {
    pub fn new(budgets: ResourceDemand, max_active: usize, wake: Arc<Notify>) -> Arc<Self> {
        Arc::new(Self {
            available: Mutex::new((budgets, 0)),
            max_active,
            wake,
        })
    }

    pub fn reserve(self: &Arc<Self>, demand: ResourceDemand) -> Option<Arc<ResourceLease>> {
        let mut state = self
            .available
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.1 == self.max_active || !demand.fits(state.0) {
            return None;
        }
        state.0.cpu -= demand.cpu;
        state.0.network -= demand.network;
        state.0.disk -= demand.disk;
        state.0.processes -= demand.processes;
        state.1 += 1;
        Some(Arc::new(ResourceLease {
            pool: self.clone(),
            demand,
        }))
    }

    pub fn available(&self) -> ResourceDemand {
        self.available
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0
    }
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        let mut state = self
            .pool
            .available
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.0.cpu += self.demand.cpu;
        state.0.network += self.demand.network;
        state.0.disk += self.demand.disk;
        state.0.processes += self.demand.processes;
        state.1 -= 1;
        drop(state);
        self.pool.wake.notify_one();
    }
}
