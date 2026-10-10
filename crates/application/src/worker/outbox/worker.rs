use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use futures::{StreamExt, stream::FuturesUnordered};
use ports::artifact_finalization::ArtifactFinalizationLookup;
use ports::events::AppEventPublisher;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::translation_cleanup::TranslationCleanupPort;
use ports::translation_links::TranslationLinkStore;
use ports::workspace::TempWorkspacePort;
use tracing::Instrument;

use super::maintenance::OutboxMaintenanceConfig;
use super::payload_handler::PayloadHandler;
use super::report::OutboxBatchReport;
use crate::error::ApplicationError;

pub struct OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactFinalizationLookup + Clone,
    U: StorageUnitOfWork,
{
    pub(super) outbox_repo: O,
    pub(super) handler: PayloadHandler<S, I, U>,
    pub(super) config: OutboxMaintenanceConfig,
    pub(super) imports: Option<Arc<dyn ports::youtube_import::YoutubeImportJournal>>,
    pub(super) translations: Option<Arc<dyn TranslationLinkStore>>,
    pub(super) resources: Option<Arc<dyn ports::outbox_resources::OutboxResourceGate>>,
    pub(super) stopping: AtomicBool,
    pub(super) active_attempts: AtomicUsize,
    pub(super) peak_active: AtomicUsize,
}

impl<O, S, I, U> OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactFinalizationLookup + Clone,
    U: StorageUnitOfWork,
{
    pub fn new(
        outbox_repo: O,
        artifact_store: S,
        artifact_index: I,
        uow: U,
        event_publisher: Arc<dyn AppEventPublisher>,
        workspace_port: Arc<dyn TempWorkspacePort>,
        config: OutboxMaintenanceConfig,
    ) -> Self {
        let handler = PayloadHandler {
            artifact_store: artifact_store.clone(),
            artifact_index,
            uow,
            event_publisher,
            workspace_port,
            translation_cleanup: None,
        };
        Self {
            outbox_repo,
            handler,
            config,
            imports: None,
            translations: None,
            resources: None,
            stopping: AtomicBool::new(false),
            active_attempts: AtomicUsize::new(0),
            peak_active: AtomicUsize::new(0),
        }
    }

    pub fn with_imports(
        mut self,
        imports: Arc<dyn ports::youtube_import::YoutubeImportJournal>,
    ) -> Self {
        self.imports = Some(imports);
        self
    }

    pub fn with_translations(mut self, translations: Arc<dyn TranslationLinkStore>) -> Self {
        self.translations = Some(translations);
        self
    }

    pub fn with_translation_cleanup(mut self, cleanup: Arc<dyn TranslationCleanupPort>) -> Self {
        self.handler.translation_cleanup = Some(cleanup);
        self
    }

    pub fn with_resource_gate(
        mut self,
        gate: Arc<dyn ports::outbox_resources::OutboxResourceGate>,
    ) -> Self {
        self.resources = Some(gate);
        self
    }

    pub fn process_pending_messages<'a>(
        &'a self,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<OutboxBatchReport, ApplicationError>> + 'a {
        let span = tracing::info_span!("outbox_batch", action = "process_pending_messages");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::OutboxBatch {
                action: "process_pending_messages",
                status: "aborted".into(),
                failed_count: 0,
            },
        );
        async move {
            let result = self.process_pending_messages_inner(limit).await;
            match &result {
                Ok(report) => {
                    let failed = report.corrupted + report.storage_errors + report.dead;
                    guard.update_summary(
                        crate::observability::execution_summary::OperationSummary::OutboxBatch {
                            action: "process_pending_messages",
                            status: if failed > 0 {
                                "completed_with_errors"
                            } else {
                                "completed"
                            }
                            .into(),
                            failed_count: failed as u64,
                        },
                    );
                }
                Err(_) => guard.summary.update_status("failed"),
            }
            result
        }
        .instrument(span)
    }

    async fn process_pending_messages_inner(
        &self,
        limit: usize,
    ) -> Result<OutboxBatchReport, ApplicationError> {
        self.config.validate()?;
        let limit = limit.min(self.config.fetch_batch);
        let mut report = OutboxBatchReport::default();
        let mut pending = FuturesUnordered::new();
        let mut claim_failure = None;
        loop {
            if report.claimed < limit
                && claim_failure.is_none()
                && !self.stopping.load(Ordering::Acquire)
            {
                let capacity = (limit - report.claimed).min(self.config.max_active - pending.len());
                if capacity > 0 {
                    let mut leases = Vec::with_capacity(capacity);
                    if let Some(gate) = &self.resources {
                        for _ in 0..capacity {
                            let Some(lease) = gate.try_acquire_disk() else {
                                break;
                            };
                            leases.push(lease);
                        }
                    }
                    let available = if self.resources.is_some() {
                        leases.len()
                    } else {
                        capacity
                    };
                    if available > 0 {
                        let ready = self
                            .outbox_repo
                            .claim_ready(available, self.config.lease_timeout)
                            .await;
                        match ready {
                            Ok(ready) => {
                                report.corrupted += ready.corrupted_isolated;
                                report.storage_errors += ready.isolation_errors;
                                report.fetched += ready.claims.len();
                                report.claimed += ready.claims.len();
                                let mut lease_iter = leases.into_iter();
                                for claim in ready.claims {
                                    let lease = lease_iter.next();
                                    pending.push(async move {
                                        let result = self.process_claim(claim).await;
                                        drop(lease);
                                        result
                                    });
                                }
                            }
                            Err(error) => claim_failure = Some(error),
                        }
                    }
                }
            }
            let Some(attempt) = pending.next().await else {
                break;
            };
            report.completed += attempt.completed;
            report.retry_scheduled += attempt.retry_scheduled;
            report.dead += attempt.dead;
            report.storage_errors += attempt.storage_errors;
            report.claim_conflicts += attempt.claim_conflicts;
        }
        if let Some(error) = claim_failure {
            return Err(error.into());
        }
        self.reconcile_ready_publications(limit).await?;
        report.max_active = self.peak_active.load(Ordering::Acquire);
        Ok(report)
    }

    async fn reconcile_ready_publications(&self, limit: usize) -> Result<(), ApplicationError> {
        let Some(translations) = &self.translations else {
            return Ok(());
        };
        for publication in translations.list_ready_publications(limit).await? {
            match translations
                .finalize_ready_publication(
                    publication.translation_id,
                    publication.result_id,
                    domain::chrono::Utc::now(),
                )
                .await
            {
                Ok(_) => tracing::info!(translation_id = %publication.translation_id,
                    result_id = %publication.result_id, "ready translation publication finalized"),
                Err(ports::error::PortError::Conflict { .. }) => tracing::debug!(
                    translation_id = %publication.translation_id, result_id = %publication.result_id,
                    "translation publication changed during reconciliation"),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
