use ports::artifact_finalization::ArtifactFinalizationLookup;
use ports::repository::{OutboxClaim, OutboxRepository};
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use std::sync::atomic::Ordering;
use tracing::Instrument;

use super::report::OutboxBatchReport;
use super::worker::OutboxWorker;

impl<O, S, I, U> OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactFinalizationLookup + Clone,
    U: StorageUnitOfWork,
{
    pub(super) async fn process_claim(&self, claim: OutboxClaim) -> OutboxBatchReport {
        struct ActiveGuard<'a>(&'a std::sync::atomic::AtomicUsize);
        impl Drop for ActiveGuard<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::AcqRel);
            }
        }
        let active = self.active_attempts.fetch_add(1, Ordering::AcqRel) + 1;
        self.peak_active.fetch_max(active, Ordering::AcqRel);
        let _active_guard = ActiveGuard(&self.active_attempts);
        let mut report = OutboxBatchReport::default();
        let message = claim.message;
        let token = claim.token;
        let started = std::time::Instant::now();
        match self.outbox_repo.claim_is_current(&message.id, &token).await {
            Ok(true) => {}
            Ok(false) => {
                report.claim_conflicts += 1;
                return report;
            }
            Err(_) => {
                report.storage_errors += 1;
                return report;
            }
        }

        let payload = self
            .handler
            .process_payload(&message.id, &token, &message.payload)
            .instrument(
                tracing::info_span!("outbox_attempt", operation_id = %message.id,
                attempt = message.attempts + 1),
            );
        tokio::pin!(payload);
        let mut heartbeat = tokio::time::interval(self.config.heartbeat_interval);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        heartbeat.tick().await;
        let outcome = loop {
            tokio::select! {
                result = &mut payload => break Some(result),
                _ = heartbeat.tick() => match self.outbox_repo.renew_claim(&message.id, &token).await {
                    Ok(true) => {},
                    Ok(false) => break None,
                    Err(_) => break None,
                }
            }
        };

        match outcome {
            Some(Ok(())) => match self.outbox_repo.mark_done_claim(&message.id, &token).await {
                Ok(true) => {
                    report.completed += 1;
                    super::attempt::completed(&message, "completed", "None", started);
                }
                Ok(false) => report.claim_conflicts += 1,
                Err(_) => report.storage_errors += 1,
            },
            Some(Err(error)) => {
                let code = super::attempt::error_code(&error);
                match self
                    .outbox_repo
                    .mark_failed_claim(&message.id, &token, &error.to_string())
                    .await
                {
                    Ok(true) => {
                        if message.attempts >= 4
                            && !matches!(
                                message.payload,
                                domain::outbox::OutboxPayload::DeleteProjectTranslation { .. }
                            )
                        {
                            report.dead += 1;
                            super::attempt::completed(&message, "dead", &code, started);
                        } else {
                            report.retry_scheduled += 1;
                            super::attempt::completed(&message, "retry_scheduled", &code, started);
                        }
                    }
                    Ok(false) => report.claim_conflicts += 1,
                    Err(_) => report.storage_errors += 1,
                }
            }
            None => {
                report.claim_conflicts += 1;
                super::attempt::completed(&message, "lease_lost", "OutboxClaimLost", started);
            }
        }
        report
    }
}
