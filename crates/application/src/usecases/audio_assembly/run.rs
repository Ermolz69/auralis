use super::{
    AudioAssemblyUseCases, AudioAssemblyView,
    error::{conflict, invalid},
};
use crate::{error::ApplicationError, worker::job_ownership::CompletionGuard};
use domain::{
    job::{Job, JobKind},
    project::ProjectId,
};
use ports::{
    PortError,
    audio_jobs::AudioAssemblyIntent,
    cancellation::{CancelHandle, CancellationToken},
    job_runtime_control::{RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome},
    job_scheduler::JobLifecycleEventKind,
    speech_audio::SpeechAudioAssemblyRequest,
};
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

impl AudioAssemblyUseCases {
    pub async fn start(
        &self,
        project: ProjectId,
        id: Uuid,
        resume: bool,
    ) -> Result<AudioAssemblyView, ApplicationError> {
        let intent = self.intent(&project, id).await?;
        let view = self.view(intent.clone()).await?;
        if self.store.publication(id).await?.is_some() {
            return Ok(view);
        }
        if self.renderer.is_none() {
            return Err(invalid(self.renderer_error.clone().unwrap_or_else(|| {
                "Managed FFmpeg is unavailable; repair the installed media tools".into()
            }))
            .into());
        }
        self.stored_plan(&intent)?;
        if self.store.active(id).await?.is_some() {
            return Err(conflict("Assembly has an active or interrupted attempt; wait or restart Auralis to reconcile").into());
        }
        if resume != self.store.latest_job(id).await?.is_some() {
            return Err(conflict(
                "Start a new assembly once; use resume after an interrupted attempt",
            )
            .into());
        }
        let attempt = Uuid::new_v4();
        let job = self
            .store
            .create_pending(
                id,
                attempt,
                Job::new(
                    project.clone(),
                    "Сборка озвучки".into(),
                    JobKind::AudioAssembly,
                ),
            )
            .await?;
        let job_id = job.id().clone();
        if let Err(error) = self
            .runtime
            .reserve(job_id.clone(), job.project_id().clone())
            .await
        {
            self.finish_error(id, attempt, &error, false).await?;
            return Err(error.into());
        }
        self.emit(JobLifecycleEventKind::Created, &job);
        let (handle, token) = CancelHandle::new();
        let (release, released) = oneshot::channel();
        let completion = Arc::new(RuntimeCompletion::new());
        let task_completion = completion.clone();
        let cases = self.clone();
        let join_handle = tokio::spawn(async move {
            let guard =
                CompletionGuard::new(cases.runtime.clone(), job.id().clone(), task_completion);
            let _cancel_on_drop = ports::cancellation::CancelOnDrop(token.clone());
            let admitted = tokio::select! { result = released => result.is_ok(), _ = token.cancelled() => false };
            let result = if admitted {
                cases.run(intent, attempt, job, token.clone()).await
            } else {
                Err(PortError::Cancelled.into())
            };
            let outcome = match result {
                Ok(()) => RuntimeTaskOutcome::Completed,
                Err(ApplicationError::Port(PortError::CleanupFailed { ref resource, .. }))
                    if resource == "ffmpeg_process" || resource == "audio_machine_admission" =>
                {
                    RuntimeTaskOutcome::CleanupUnconfirmed
                }
                Err(error) => {
                    let error = PortError::Unexpected {
                        message: error.to_string(),
                    };
                    if let Err(error) = cases
                        .finish_error(id, attempt, &error, token.is_cancelled())
                        .await
                    {
                        tracing::error!(error=%error, "Audio assembly requires recovery");
                    }
                    if token.is_cancelled() {
                        RuntimeTaskOutcome::Cancelled
                    } else {
                        RuntimeTaskOutcome::ApplicationFailed
                    }
                }
            };
            guard.complete(outcome)
        });
        let task = RuntimeTask {
            cancel: handle,
            join_handle,
            completion,
        };
        if let Err(error) = self.runtime.attach_task(job_id.clone(), task).await {
            error.task.cancel.cancel();
            drop(release);
            let _ = error.task.join_handle.await;
            self.runtime.rollback_runtime_start(&job_id).await?;
            return Err(error.source.into());
        }
        release
            .send(())
            .map_err(|_| conflict("Assembly task stopped before runtime admission"))?;
        self.state(project, id).await
    }

    async fn run(
        &self,
        intent: AudioAssemblyIntent,
        attempt: Uuid,
        mut job: Job,
        cancel: CancellationToken,
    ) -> Result<(), ApplicationError> {
        let revision = job.revision();
        job.start()?;
        self.store.update(&job, attempt, revision).await?;
        self.emit(JobLifecycleEventKind::Started, &job);
        self.stage(
            &mut job,
            attempt,
            5,
            domain::dubbing::DubbingPipelineStage::ValidateSource,
            "Проверка исходной озвучки",
        )
        .await?;
        let checked = self.recheck(&intent, cancel.clone()).await?;
        if cancel.is_cancelled() {
            return Err(PortError::Cancelled.into());
        }
        self.stage(
            &mut job,
            attempt,
            35,
            domain::dubbing::DubbingPipelineStage::PostprocessAudio,
            "Сборка полного аудио",
        )
        .await?;
        let plan = checked.plan;
        let expected = domain::speech::timeline::plan_audio_timeline(&plan.timeline)
            .map_err(|_| invalid("Reviewed assembly timing now conflicts"))?;
        let result = self
            .renderer
            .as_ref()
            .ok_or_else(|| invalid("Managed FFmpeg is unavailable"))?
            .assemble_speech(
                SpeechAudioAssemblyRequest {
                    source: checked.inspection.source,
                    mode: plan.timeline.mode,
                    source_duration_ms: plan.timeline.source_duration_ms,
                    pause_after_ms: plan
                        .timeline
                        .utterances
                        .iter()
                        .map(|utterance| utterance.pause_after_ms)
                        .collect(),
                },
                cancel.clone(),
            )
            .await?;
        if result.sample_rate != 48000
            || result.channels != expected.channels
            || result.frames != expected.frame_count
        {
            self.cleanup_unpublished(&[], Some(result.staging_root))
                .await?;
            return Err(
                invalid("Verified assembly measurements differ from the reviewed plan").into(),
            );
        }
        if let Err(error) = self
            .stage(
                &mut job,
                attempt,
                90,
                domain::dubbing::DubbingPipelineStage::ExportResult,
                "Проверка и публикация WAV",
            )
            .await
        {
            self.cleanup_unpublished(&[], Some(result.staging_root))
                .await?;
            return Err(error);
        }
        self.publish_result(&intent, attempt, &job, result, cancel)
            .await
    }
}
