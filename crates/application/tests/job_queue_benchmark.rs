#![allow(clippy::unwrap_used)]

#[path = "support/job_queue.rs"]
mod support;

use jobs::queue::JobQueueConfig;
use ports::{
    job_execution::{ExecutionContext, ExecutionDescriptor, JobExecutorPort, QueuedExecution},
    job_resources::ResourceDemand,
    job_runtime_control::RuntimeTaskOutcome,
    job_scheduler::JobSchedulerPort,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct FixtureExecutor {
    manager: Arc<jobs::manager::JobManager>,
    active: AtomicUsize,
    peak: AtomicUsize,
    done: tokio::sync::mpsc::UnboundedSender<()>,
    started: tokio::sync::mpsc::UnboundedSender<domain::job::JobId>,
}

#[async_trait::async_trait]
impl JobExecutorPort for FixtureExecutor {
    fn validate(&self, descriptor: &ExecutionDescriptor) -> Result<(), ports::PortError> {
        descriptor.validate()
    }
    async fn execute(
        &self,
        execution: QueuedExecution,
        context: ExecutionContext,
    ) -> RuntimeTaskOutcome {
        let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
        self.peak.fetch_max(active, Ordering::AcqRel);
        self.started.send(execution.job.id().clone()).unwrap();
        let cancelled = tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(40)) => false,
            _ = context.cancellation.cancelled() => true,
        };
        self.active.fetch_sub(1, Ordering::AcqRel);
        if !cancelled {
            self.manager.complete_job(execution.job.id()).await.unwrap();
        }
        self.done.send(()).unwrap();
        if cancelled {
            RuntimeTaskOutcome::Cancelled
        } else {
            RuntimeTaskOutcome::Completed
        }
    }
}

#[tokio::test]
#[ignore = "explicit, bounded fixture comparison"]
async fn compare_sequential_and_parallel_fixtures() {
    let mut rows = Vec::new();
    for slots in [1, 2] {
        for repetition in 0..3 {
            let harness = support::Harness::new(JobQueueConfig {
                budgets: ResourceDemand {
                    cpu: 2,
                    network: slots,
                    disk: 2,
                    processes: 2,
                },
                ..Default::default()
            })
            .await;
            let (done, mut receive) = tokio::sync::mpsc::unbounded_channel();
            let (started, mut executing) = tokio::sync::mpsc::unbounded_channel();
            let executor = Arc::new(FixtureExecutor {
                manager: harness.manager.clone(),
                active: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                done,
                started,
            });
            harness
                .manager
                .start_dispatcher(executor.clone())
                .await
                .unwrap();
            let start = Instant::now();
            for _ in 0..12 {
                harness.admit(support::NETWORK).await;
            }
            for _ in 0..12 {
                receive.recv().await.unwrap();
            }
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            for _ in 0..12 {
                executing.recv().await.unwrap();
            }
            let cancel_id = harness.admit(support::NETWORK).await;
            harness
                .wait_status(&cancel_id, domain::job::JobStatus::Running)
                .await;
            assert_eq!(executing.recv().await.unwrap(), cancel_id);
            let cancel_start = Instant::now();
            harness.manager.cancel_job(&cancel_id).await.unwrap();
            let cancel_ms = cancel_start.elapsed().as_secs_f64() * 1000.0;
            rows.push(serde_json::json!({ "slots": slots, "repetition": repetition + 1, "tasks": 12, "elapsed_ms": elapsed_ms, "peak": executor.peak.load(Ordering::Acquire), "cancel_ms": cancel_ms }));
            harness.shutdown().await;
        }
    }
    let report = serde_json::json!({"protocol_version": 2, "cancellation_boundary": "executor entered useful fixture work", "workload": "12 SQLite-backed jobs with 40ms cancellable asynchronous local fixture waits", "repetitions": 3, "results": rows});
    let path = std::path::Path::new("../../target/job-queue/fixture-comparison-started.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
