use super::manager::JobManager;
use super::outcome::classify_outcome;

impl JobManager {
    pub(super) async fn drain_all_internal(
        &self,
        deadline: std::time::Duration,
    ) -> Result<ports::job_runtime_control::RuntimeShutdownReport, ports::error::PortError> {
        let deadline = self.queue.as_ref().map_or(deadline, |queue| {
            deadline.min(queue.config.shutdown_timeout)
        });
        let start_time = std::time::Instant::now();
        let expires_at = tokio::time::Instant::now() + deadline;
        let mut reaper_entries = Vec::new();
        let mut report = ports::job_runtime_control::RuntimeShutdownReport::default();

        if !self.stop_dispatcher(expires_at).await {
            report.cleanup_deferred_count += 1;
        }

        let entries = match self.runtime_registry.try_close() {
            Some(e) => e,
            None => return Err(ports::error::PortError::AlreadyStopped),
        };

        for (job_id, entry) in entries {
            match entry {
                super::runtime_registry::JobRuntimeEntry::Managed { .. } => {
                    unreachable!("managed tasks remain registered")
                }
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

        let managed = self.managed_job_ids();
        let managed_report = self
            .cancel_managed_before(&managed, expires_at, false)
            .await?;
        if let Some(queue) = &self.queue {
            queue.cpu.close();
        }
        for outcome in managed_report.jobs.values() {
            use ports::job_runtime_control::RuntimeCleanupOutcome;
            match outcome {
                RuntimeCleanupOutcome::Completed => report.completed_count += 1,
                RuntimeCleanupOutcome::CooperativeCancelled => {
                    report.cooperative_cancelled_count += 1
                }
                RuntimeCleanupOutcome::Panicked => report.panicked_count += 1,
                RuntimeCleanupOutcome::JoinFailed => report.join_failed_count += 1,
                RuntimeCleanupOutcome::Unconfirmed => report.unconfirmed_count += 1,
                RuntimeCleanupOutcome::ReservationRemoved => report.reservation_removed_count += 1,
                RuntimeCleanupOutcome::ForcedAborted => report.forced_aborted_count += 1,
                RuntimeCleanupOutcome::Missing => {}
            }
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
