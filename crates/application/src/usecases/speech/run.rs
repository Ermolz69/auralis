use super::*;
use crate::error::ApplicationError;
use domain::{
    job::{Job, JobError, JobKind, JobStatus},
    media::ArtifactKind,
    project::ProjectId,
};
use ports::{
    cancellation::{CancelHandle, CancellationToken},
    error::PortError,
    job_runtime_control::{RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome},
    job_scheduler::JobLifecycleEventKind,
    speech_run::SpeechRunIntent,
    speech_workflow::{SpeechExecutionResult, SpeechPublication, SpeechPublishedFile},
};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

impl SpeechUseCases {
    pub async fn start(
        &self,
        project: ProjectId,
        run_id: Uuid,
        resume: bool,
    ) -> Result<SpeechRunView, ApplicationError> {
        let intent = self.intent(&project, run_id).await?;
        self.store.close_interrupted().await?;
        if self.store.publication(run_id).await?.is_some() {
            return self.view(intent).await;
        }
        if self.store.active_for_run(run_id).await?.is_some() {
            return Err(conflict("Speech run already has an active attempt").into());
        }
        let previous = self.store.latest_job(run_id).await?;
        if resume != previous.is_some() {
            return Err(
                conflict("Use start for a new run and resume for an interrupted run").into(),
            );
        }
        self.check_intent(&intent).await?;
        let attempt_id = Uuid::new_v4();
        let job = self
            .store
            .create_pending(
                Job::new(project, "Синтез речи".into(), JobKind::Speech),
                run_id,
                attempt_id,
            )
            .await?;
        let id = job.id().clone();
        if let Err(e) = self
            .runtime
            .reserve(id.clone(), job.project_id().clone())
            .await
        {
            self.finish_error(run_id, attempt_id, &e, false).await?;
            return Err(e.into());
        }
        self.emit(JobLifecycleEventKind::Created, &job);
        let (handle, token) = CancelHandle::new();
        let (release, released) = oneshot::channel();
        let completion = Arc::new(RuntimeCompletion::new());
        let task_completion = completion.clone();
        let cases = self.clone();
        let join_handle = tokio::spawn(async move {
            let guard = CompletionGuard {
                runtime: cases.runtime.clone(),
                job_id: job.id().clone(),
                completion: task_completion,
            };
            let admitted =
                tokio::select! { r = released => r.is_ok(), _ = token.cancelled() => false };
            let result = if admitted {
                cases.run(intent, attempt_id, job, token.clone()).await
            } else {
                Err(PortError::Cancelled.into())
            };
            let outcome = match result {
                Ok(true) => RuntimeTaskOutcome::Completed,
                Ok(false) => RuntimeTaskOutcome::Cancelled,
                Err(ApplicationError::Port(ref error))
                    if matches!(error, PortError::CleanupFailed { .. }) =>
                {
                    RuntimeTaskOutcome::CleanupUnconfirmed
                }
                Err(error) => {
                    let cause = PortError::Unexpected {
                        message: error.to_string(),
                    };
                    if let Err(e) = cases
                        .finish_error(run_id, attempt_id, &cause, token.is_cancelled())
                        .await
                    {
                        tracing::error!(error=%e, "Speech attempt requires recovery");
                    }
                    if token.is_cancelled() {
                        RuntimeTaskOutcome::Cancelled
                    } else {
                        RuntimeTaskOutcome::ApplicationFailed
                    }
                }
            };
            guard.completion.record_outcome(outcome)
        });
        let task = RuntimeTask {
            cancel: handle,
            join_handle,
            completion,
        };
        if let Err(error) = self.runtime.attach_task(id.clone(), task).await {
            error.task.cancel.cancel();
            drop(release);
            let _ = error.task.join_handle.await;
            self.runtime.rollback_runtime_start(&id).await?;
            return Err(error.source.into());
        }
        release
            .send(())
            .map_err(|_| conflict("Speech task stopped before admission"))?;
        self.state(
            self.store
                .get_run(run_id)
                .await?
                .ok_or_else(|| invalid("Speech intent disappeared"))?
                .project_id,
            run_id,
        )
        .await
    }

    async fn run(
        &self,
        intent: SpeechRunIntent,
        attempt_id: Uuid,
        mut job: Job,
        token: CancellationToken,
    ) -> Result<bool, ApplicationError> {
        let previous = job.revision();
        job.start()?;
        self.store.start(&job, attempt_id, previous).await?;
        self.emit(JobLifecycleEventKind::Started, &job);
        let observer = Arc::new(Progress {
            cases: self.clone(),
            job: Mutex::new(job),
            total: intent.expected_chunks,
        });
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| invalid("TTS runtime unavailable"))?;
        match engine
            .execute(intent.clone(), attempt_id, observer.clone(), token.clone())
            .await?
        {
            SpeechExecutionResult::Cancelled => {
                self.finish_error(intent.run_id, attempt_id, &PortError::Cancelled, true)
                    .await?;
                Ok(false)
            }
            SpeechExecutionResult::Completed { files, duration_ms } => {
                if token.is_cancelled() {
                    return Err(PortError::Cancelled.into());
                }
                let mut staged = Vec::new();
                let mut published = Vec::new();
                for (q, file) in files.iter().enumerate() {
                    let kind = match file.kind.as_str() {
                        "wav" => ArtifactKind::GeneratedSpeech,
                        "segment" => ArtifactKind::GeneratedSpeechSegment,
                        "manifest" => ArtifactKind::SpeechManifest,
                        _ => return Err(invalid("Unknown speech output").into()),
                    };
                    let name = format!(
                        "speech-{q}.{}",
                        if file.kind == "manifest" {
                            "json"
                        } else {
                            "wav"
                        }
                    );
                    let artifact = self
                        .artifacts
                        .import_external_file(&intent.project_id, kind, &file.path, Some(&name))
                        .await?;
                    if !self
                        .artifacts
                        .verify_storage_digest(
                            &artifact.staging_key,
                            artifact.size_bytes,
                            &file.sha256,
                        )
                        .await?
                    {
                        return Err(invalid("Speech staging hash mismatch").into());
                    }
                    published.push(SpeechPublishedFile {
                        artifact_id: artifact.artifact.id.clone(),
                        kind: file.kind.clone(),
                        sha256: file.sha256.clone(),
                        size_bytes: artifact.size_bytes,
                    });
                    staged.push(artifact);
                }
                let job = observer.job.lock().await;
                self.store
                    .publish(
                        &job,
                        &SpeechPublication {
                            run_id: intent.run_id,
                            attempt_id,
                            duration_ms,
                            files: published,
                            ready: false,
                        },
                        &staged,
                    )
                    .await?;
                drop(job);
                for _ in 0..120 {
                    if token.is_cancelled() {
                        return Err(PortError::Cancelled.into());
                    }
                    if let Some(job) = self.store.complete_publication(intent.run_id).await? {
                        self.emit(JobLifecycleEventKind::Completed, &job);
                        return Ok(true);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(conflict("Audio is saved; publication is waiting for recovery").into())
            }
        }
    }

    async fn finish_error(
        &self,
        run_id: Uuid,
        attempt_id: Uuid,
        error: &PortError,
        cancelled: bool,
    ) -> Result<(), ApplicationError> {
        if !cancelled && self.store.publication(run_id).await?.is_some() {
            return Ok(());
        }
        if cancelled {
            self.store
                .discard_unready_publication(run_id, attempt_id)
                .await?;
        }
        let Some(active) = self.store.active_for_run(run_id).await? else {
            return Ok(());
        };
        if active.attempt_id != attempt_id {
            return Err(conflict("Stale speech failure").into());
        }
        let mut job = active.job;
        if matches!(
            job.status(),
            JobStatus::Completed | JobStatus::Cancelled | JobStatus::Failed
        ) {
            self.store.close_interrupted().await?;
            return Ok(());
        }
        let previous = job.revision();
        if cancelled {
            job.cancel()?;
        } else {
            job.mark_failed(JobError::new("speech_failed", error.to_string(), false))?;
        }
        self.store.finish(&job, attempt_id, previous).await?;
        self.emit(
            if cancelled {
                JobLifecycleEventKind::Cancelled
            } else {
                JobLifecycleEventKind::Failed
            },
            &job,
        );
        Ok(())
    }

    pub async fn recover(&self) -> Result<(), ApplicationError> {
        self.store.close_interrupted().await?;
        for active in self.store.list_active().await? {
            let cancelled = active.job.status() == &JobStatus::Cancelling;
            self.finish_error(
                active.run_id,
                active.attempt_id,
                &conflict("Auralis stopped during speech execution; resume explicitly"),
                cancelled,
            )
            .await?;
        }
        Ok(())
    }
}
