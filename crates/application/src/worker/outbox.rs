use std::sync::Arc;
use std::time::Duration;

use domain::media::ArtifactState;
use domain::outbox::OutboxPayload;
use ports::artifact_index::ArtifactIndex;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct OutboxWorker<O, S, I>
where
    O: OutboxRepository,
    S: ArtifactStore,
    I: ArtifactIndex,
{
    outbox_repo: O,
    artifact_store: S,
    artifact_index: I,
}

impl<O, S, I> OutboxWorker<O, S, I>
where
    O: OutboxRepository,
    S: ArtifactStore,
    I: ArtifactIndex,
{
    pub fn new(outbox_repo: O, artifact_store: S, artifact_index: I) -> Self {
        Self {
            outbox_repo,
            artifact_store,
            artifact_index,
        }
    }

    pub async fn process_pending_messages(&self, limit: usize) -> Result<usize, ApplicationError> {
        let messages = self.outbox_repo.fetch_pending(limit).await?;
        let count = messages.len();
        if count == 0 {
            return Ok(0);
        }

        let worker_id = uuid::Uuid::new_v4().to_string();

        for message in messages {
            // Lock the message
            if let Err(_e) = self
                .outbox_repo
                .mark_processing(&message.id, &worker_id)
                .await
            {
                // Ignore lock error, maybe another worker took it
                continue;
            }

            let result = self.process_payload(&message.payload).await;

            match result {
                Ok(_) => {
                    let _ = self.outbox_repo.mark_done(&message.id).await;
                }
                Err(e) => {
                    let _ = self
                        .outbox_repo
                        .mark_failed(&message.id, &e.to_string())
                        .await;
                }
            }
        }

        Ok(count)
    }

    async fn process_payload(&self, payload: &OutboxPayload) -> Result<(), ApplicationError> {
        match payload {
            OutboxPayload::FinalizeStagedArtifact {
                artifact_id,
                staging_key,
                final_key,
            } => {
                // 1. Move file
                self.artifact_store
                    .finalize_staged_artifact(staging_key, final_key)
                    .await?;

                // 2. Update artifact state in index
                self.artifact_index
                    .update_state(
                        artifact_id,
                        ArtifactState::Ready,
                        Some(domain::chrono::Utc::now()),
                    )
                    .await?;
            }
            OutboxPayload::DeleteStorageKey { storage_key } => {
                self.artifact_store.delete_storage_key(storage_key).await?;
            }
            OutboxPayload::DeleteProjectArtifactDir { project_id } => {
                self.artifact_store.delete_project_dir(project_id).await?;
            }
            OutboxPayload::DeleteTempPath { path } => {
                let _ = tokio::fs::remove_file(path).await;
            }
        }

        Ok(())
    }

    pub async fn run_loop(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.process_pending_messages(10).await {
                        eprintln!("OutboxWorker error: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    println!("OutboxWorker shutting down...");
                    break;
                }
            }
        }
    }
}
