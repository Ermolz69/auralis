use ports::{PortError, job_resources::CpuExecutorPort};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use tokio::sync::Notify;

use super::resources::ResourceLease;

type Work = Box<dyn FnOnce() + Send + 'static>;

pub(crate) struct CpuPool {
    sender: Mutex<Option<mpsc::SyncSender<Work>>>,
}

impl CpuPool {
    pub fn new(workers: u32, backlog: usize) -> Result<Arc<Self>, PortError> {
        let (sender, receiver) = mpsc::sync_channel::<Work>(backlog);
        let receiver = Arc::new(Mutex::new(receiver));
        for index in 0..workers {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name(format!("auralis-cpu-{index}"))
                .spawn(move || {
                    loop {
                        let next = receiver
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .recv();
                        let Ok(work) = next else {
                            break;
                        };
                        work();
                    }
                })
                .map_err(|error| PortError::Io {
                    message: error.to_string(),
                })?;
        }
        Ok(Arc::new(Self {
            sender: Mutex::new(Some(sender)),
        }))
    }

    pub fn close(&self) {
        self.sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}

pub(crate) struct JobCpuExecutor {
    pool: Arc<CpuPool>,
    lease: Arc<ResourceLease>,
    enabled: bool,
    outstanding: Arc<AtomicUsize>,
    stopped: Arc<Notify>,
}

impl JobCpuExecutor {
    pub fn new(pool: Arc<CpuPool>, lease: Arc<ResourceLease>, enabled: bool) -> Arc<Self> {
        Arc::new(Self {
            pool,
            lease,
            enabled,
            outstanding: Arc::new(AtomicUsize::new(0)),
            stopped: Arc::new(Notify::new()),
        })
    }

    pub async fn drain(&self) {
        loop {
            let notified = self.stopped.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.outstanding.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }
}

#[async_trait::async_trait]
impl CpuExecutorPort for JobCpuExecutor {
    async fn run(&self, work: Work) -> Result<(), PortError> {
        if !self.enabled {
            return Err(PortError::InvalidResourceRequest {
                message: "CPU work requires a CPU reservation".into(),
            });
        }
        // One closure per job prevents a single reservation from oversubscribing CPU.
        if self
            .outstanding
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(PortError::Busy {
                message: "This job already has CPU work in flight".into(),
            });
        }
        let (send, receive) = tokio::sync::oneshot::channel();
        let lease = self.lease.clone();
        let outstanding = self.outstanding.clone();
        let stopped = self.stopped.clone();
        let task = Box::new(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            drop(lease);
            outstanding.store(0, Ordering::Release);
            stopped.notify_waiters();
            let _ = send.send(result.is_ok());
        });
        let accepted = self
            .pool
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|sender| sender.try_send(task).is_ok());
        if !accepted {
            self.outstanding.store(0, Ordering::Release);
            self.stopped.notify_waiters();
            return Err(PortError::Busy {
                message: "CPU executor backlog is full or closed".into(),
            });
        }
        match receive.await {
            Ok(true) => Ok(()),
            _ => Err(PortError::Unexpected {
                message: "CPU worker panicked".into(),
            }),
        }
    }
}
