use domain::job::JobId;
use ports::{
    PortError,
    job_runtime_control::{RuntimeCleanupOutcome, RuntimeCleanupReport, RuntimeTaskOutcome},
    job_scheduler::{JobLifecycleEventKind, JobSchedulerPort},
};
use std::collections::HashMap;

use crate::manager::{JobManager, runtime_registry::JobRuntimeEntry};

impl JobManager {
    pub(crate) fn managed_job_ids(&self) -> Vec<JobId> {
        self.runtime_registry
            .lock_entries()
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                matches!(entry, JobRuntimeEntry::Managed { .. }).then_some(id.clone())
            })
            .collect()
    }

    pub(crate) async fn cancel_managed_before(
        &self,
        ids: &[JobId],
        deadline: tokio::time::Instant,
        deleted: bool,
    ) -> Result<RuntimeCleanupReport, PortError> {
        let mut waits = futures::stream::FuturesUnordered::new();
        for id in ids {
            let id = id.clone();
            waits.push(async move {
                let stop = async {
                    if !deleted {
                        self.mutate_job(&id, JobLifecycleEventKind::Cancelling, |job| {
                            if matches!(
                                job.status(),
                                domain::job::JobStatus::Completed
                                    | domain::job::JobStatus::Failed
                                    | domain::job::JobStatus::Cancelled
                            ) {
                                Ok(())
                            } else {
                                job.request_cancellation()
                            }
                        })
                        .await?;
                    }
                    let completion = {
                        let registry = self.runtime_registry.lock_entries();
                        match registry.entries.get(&id) {
                            Some(JobRuntimeEntry::Managed { task, .. }) => {
                                task.cancel.cancel();
                                Some(task.completion.clone())
                            }
                            _ => None,
                        }
                    };
                    let outcome = match completion {
                        Some(completion) => completion.wait().await,
                        None => RuntimeTaskOutcome::Cancelled,
                    };
                    if outcome == RuntimeTaskOutcome::CleanupUnconfirmed {
                        return Ok(RuntimeCleanupOutcome::Unconfirmed);
                    }
                    if !deleted {
                        match self.cancel_job(&id).await {
                            Ok(_) => {}
                            // A completed worker won the transaction while shutdown requested cancellation.
                            Err(_)
                                if self.get_job(&id).await?.is_some_and(|job| {
                                    matches!(
                                        job.status,
                                        domain::job::JobStatus::Completed
                                            | domain::job::JobStatus::Failed
                                    )
                                }) => {}
                            Err(error) => return Err(error),
                        }
                    }
                    Ok(match outcome {
                        RuntimeTaskOutcome::Completed | RuntimeTaskOutcome::DeletedNoOp => {
                            RuntimeCleanupOutcome::Completed
                        }
                        RuntimeTaskOutcome::Cancelled => {
                            RuntimeCleanupOutcome::CooperativeCancelled
                        }
                        RuntimeTaskOutcome::Panicked => RuntimeCleanupOutcome::Panicked,
                        RuntimeTaskOutcome::ApplicationFailed
                        | RuntimeTaskOutcome::RecoveryRequired => RuntimeCleanupOutcome::JoinFailed,
                        RuntimeTaskOutcome::CleanupUnconfirmed => {
                            RuntimeCleanupOutcome::Unconfirmed
                        }
                    })
                };
                let outcome = match tokio::time::timeout_at(deadline, stop).await {
                    Ok(result) => result,
                    Err(_) => Ok(RuntimeCleanupOutcome::Unconfirmed),
                };
                (id, outcome)
            });
        }
        let mut report = RuntimeCleanupReport {
            jobs: HashMap::new(),
        };
        let mut failure = None;
        while let Some((id, result)) = futures::StreamExt::next(&mut waits).await {
            match result {
                Ok(outcome) => {
                    report.jobs.insert(id, outcome);
                }
                Err(error) => {
                    report.jobs.insert(id, RuntimeCleanupOutcome::Unconfirmed);
                    failure = Some(error);
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(report)
    }
}
