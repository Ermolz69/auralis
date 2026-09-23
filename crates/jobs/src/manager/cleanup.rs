use super::manager::JobManager;
use super::outcome::{classify_cleanup_outcome, classify_outcome};
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
        let mut report = ports::job_runtime_control::RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        };

        let mut abort_handles = std::collections::HashMap::new();
        let mut reaper = futures::stream::FuturesUnordered::new();

        {
            let mut registry = self.runtime_registry.lock_entries();
            for job_id in job_ids {
                match registry.entries.remove(job_id) {
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

    pub(super) async fn drain_all_internal(
        &self,
        deadline: std::time::Duration,
    ) -> Result<ports::job_runtime_control::RuntimeShutdownReport, ports::error::PortError> {
        let start_time = std::time::Instant::now();
        let expires_at = tokio::time::Instant::now() + deadline;
        let mut reaper_entries = Vec::new();
        let mut report = ports::job_runtime_control::RuntimeShutdownReport::default();

        let entries = match self.runtime_registry.try_close() {
            Some(e) => e,
            None => return Err(ports::error::PortError::AlreadyStopped),
        };

        for (job_id, entry) in entries {
            match entry {
                super::runtime_registry::JobRuntimeEntry::Reserved { .. } => {
                    report.reservation_removed_count += 1;
                    self.cache.remove(&job_id).await;
                    self.mutation_locks.remove_if_unused(&job_id)?;
                }
                super::runtime_registry::JobRuntimeEntry::Attached { task, .. } => {
                    task.cancel.cancel();
                    reaper_entries.push((job_id, task));
                }
            }
        }

        let mut abort_handles = std::collections::HashMap::new();
        let mut stream = futures::stream::FuturesUnordered::new();

        for (job_id, task) in reaper_entries {
            abort_handles.insert(job_id.clone(), task.join_handle.abort_handle());
            stream.push(async move {
                let res = task.join_handle.await;
                (job_id, res)
            });
        }

        // 1. Cooperative Wait Phase
        let elapsed = start_time.elapsed();
        let total_budget = deadline
            .checked_sub(elapsed)
            .unwrap_or(std::time::Duration::ZERO);

        let cooperative_timeout = if total_budget > std::time::Duration::from_millis(500) {
            total_budget.mul_f64(0.8)
        } else {
            std::time::Duration::ZERO
        };

        if !abort_handles.is_empty() && cooperative_timeout > std::time::Duration::ZERO {
            let timeout_fut = tokio::time::sleep(cooperative_timeout);
            tokio::pin!(timeout_fut);

            loop {
                tokio::select! {
                    _ = &mut timeout_fut => {
                        break;
                    }
                    res_opt = futures::StreamExt::next(&mut stream) => {
                        match res_opt {
                            Some((job_id, join_res)) => {
                                abort_handles.remove(&job_id);
                                classify_outcome(join_res, &mut report, false);

                                if !self.evict_stopped_before(&job_id, expires_at).await? { report.cleanup_deferred_count += 1; }
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 2. Forced Abort Phase
        if !abort_handles.is_empty() {
            for abort_handle in abort_handles.values() {
                abort_handle.abort();
            }

            let abort_budget = deadline
                .checked_sub(start_time.elapsed())
                .unwrap_or(std::time::Duration::ZERO);
            let timeout_fut = tokio::time::sleep(abort_budget);
            tokio::pin!(timeout_fut);

            while !abort_handles.is_empty() && abort_budget > std::time::Duration::ZERO {
                tokio::select! {
                    _ = &mut timeout_fut => break,
                    result = futures::StreamExt::next(&mut stream) => {
                        let Some((job_id, join_res)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        classify_outcome(join_res, &mut report, true);

                        if !self.evict_stopped_before(&job_id, expires_at).await? { report.cleanup_deferred_count += 1; }
                    }
                }
            }

            report.unconfirmed_count += abort_handles.len();
        }

        Ok(report)
    }
}
