use super::manager::JobManager;
use super::outcome::classify_cleanup_outcome;
use domain::job::JobId as DomainJobId;
use ports::error::PortError;

impl JobManager {
    pub(super) async fn evict_stopped_before(
        &self,
        job_id: &DomainJobId,
        deadline: tokio::time::Instant,
    ) -> Result<bool, PortError> {
        let cleanup = async {
            let lock = self.mutation_locks.get_lock(job_id)?;
            let guard = lock.lock().await;
            self.cache.remove(job_id).await;
            drop(guard);
            self.mutation_locks.release_if_unused(job_id, &lock)
        };
        match tokio::time::timeout_at(deadline, cleanup).await {
            Ok(result) => result.map(|_| true),
            Err(_) => Ok(false),
        }
    }

    pub(super) async fn cancel_and_evict_jobs_internal(
        &self,
        job_ids: &[domain::job::JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError> {
        let managed: Vec<_> = self
            .managed_job_ids()
            .into_iter()
            .filter(|id| job_ids.contains(id))
            .collect();
        let timeout = self
            .queue
            .as_ref()
            .map_or(std::time::Duration::from_secs(7), |queue| {
                queue.config.cleanup_timeout
            });
        let mut report = self
            .cancel_managed_before(&managed, tokio::time::Instant::now() + timeout, true)
            .await?;

        let mut abort_handles = std::collections::HashMap::new();
        let mut reaper = futures::stream::FuturesUnordered::new();

        {
            let mut registry = self.runtime_registry.lock_entries();
            for job_id in job_ids {
                if managed.contains(job_id) {
                    continue;
                }
                match registry.entries.remove(job_id) {
                    Some(super::runtime_registry::JobRuntimeEntry::Managed { .. }) => {
                        unreachable!("managed tasks were excluded")
                    }
                    Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                        task.cancel.cancel();
                        let job_id = job_id.clone();
                        abort_handles.insert(job_id.clone(), task.join_handle.abort_handle());
                        reaper.push(async move {
                            let join_result = task.join_handle.await;
                            (job_id, join_result)
                        });
                    }
                    Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                        report.jobs.insert(
                            job_id.clone(),
                            ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved,
                        );
                    }
                    None => {
                        report.jobs.insert(
                            job_id.clone(),
                            ports::job_runtime_control::RuntimeCleanupOutcome::Missing,
                        );
                    }
                }
            }
        }

        if !abort_handles.is_empty() {
            let cooperative_timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
            tokio::pin!(cooperative_timeout);

            while !abort_handles.is_empty() {
                tokio::select! {
                    _ = &mut cooperative_timeout => break,
                    result = futures::StreamExt::next(&mut reaper) => {
                        let Some((job_id, join_result)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        report.jobs.insert(
                            job_id,
                            classify_cleanup_outcome(join_result, false),
                        );
                    }
                }
            }
        }

        for abort_handle in abort_handles.values() {
            abort_handle.abort();
        }

        if !abort_handles.is_empty() {
            let abort_timeout = tokio::time::sleep(std::time::Duration::from_secs(2));
            tokio::pin!(abort_timeout);

            while !abort_handles.is_empty() {
                tokio::select! {
                    _ = &mut abort_timeout => break,
                    result = futures::StreamExt::next(&mut reaper) => {
                        let Some((job_id, join_result)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        report.jobs.insert(
                            job_id,
                            classify_cleanup_outcome(join_result, true),
                        );
                    }
                }
            }
        }

        for job_id in abort_handles.keys() {
            report.jobs.insert(
                job_id.clone(),
                ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed,
            );
        }

        if !abort_handles.is_empty() {
            let cache = self.cache.clone();
            let locks = self.mutation_locks.clone();
            tokio::spawn(async move {
                while let Some((job_id, _join_result)) = futures::StreamExt::next(&mut reaper).await
                {
                    cache.remove(&job_id).await;
                    if let Err(_err) = locks.remove_if_unused(&job_id) {
                        tracing::error!(
                            error = %common::observability::redaction::DiagnosticError {
                                kind: "JobMutationLockCleanupFailed",
                                code: None,
                                retryable: false,
                            },
                            "failed to cleanup job mutation lock"
                        );
                    }
                }
            });
        }

        // Evict cache and locks only for fully confirmed stopped jobs
        for job_id in job_ids {
            let outcome = report.jobs.get(job_id);
            if let Some(ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed) = outcome {
                continue;
            }

            let lock = self.mutation_locks.get_lock(job_id)?;
            let _guard = lock.lock().await;

            self.cache.remove(job_id).await;

            drop(_guard);
            self.mutation_locks.release_if_unused(job_id, &lock)?;
        }

        Ok(report)
    }
}
