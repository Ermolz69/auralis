use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobProgress, JobStatus};
use domain::project::ProjectId;
use ports::job_scheduler::JobSchedulerPort;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

pub struct MockDubbingPipelineRunner<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    job_scheduler: Arc<dyn JobSchedulerPort>,
    project_repo: R,
    subtitle_source: V,
    storage_uow: T,
    artifact_store: S,
    target_dir_base: std::path::PathBuf,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> MockDubbingPipelineRunner<R, V, T, S>
{
    pub fn new(
        job_scheduler: Arc<dyn JobSchedulerPort>,
        project_repo: R,
        subtitle_source: V,
        storage_uow: T,
        artifact_store: S,
        target_dir_base: std::path::PathBuf,
    ) -> Self {
        Self {
            job_scheduler,
            project_repo,
            subtitle_source,
            storage_uow,
            artifact_store,
            target_dir_base,
        }
    }

    pub fn spawn(self, job_id: JobId, project_id: ProjectId) {
        tokio::spawn(async move {
            self.run(job_id, project_id).await;
        });
    }

    #[allow(clippy::collapsible_if)]
    async fn run(&self, job_id: JobId, project_id: ProjectId) {
        let stages = vec![
            (DubbingPipelineStage::ValidateSource, 10, 500),
            (DubbingPipelineStage::FetchMetadata, 25, 600),
            (DubbingPipelineStage::DownloadMedia, 45, 800),
        ];

        for (stage, percent, delay_ms) in stages {
            // Check cancellation before each stage
            if let Ok(Some(job)) = self.job_scheduler.get_job(&job_id).await {
                if job.status == JobStatus::Cancelled {
                    return;
                }
            }

            let progress = JobProgress {
                percent: percent as u8,
                message: format!("Mock stage: {:?}", stage),
                current_step: Some(format!("{:?}", stage)),
                processed_items: None,
                total_items: None,
            };

            let _ = self
                .job_scheduler
                .update_job_stage(&job_id, stage.clone(), progress)
                .await;

            sleep(Duration::from_millis(delay_ms)).await;
        }

        // ExtractOrGenerateTranscript stage (Real work: Subtitles Import)
        if let Ok(Some(job)) = self.job_scheduler.get_job(&job_id).await {
            if job.status == JobStatus::Cancelled {
                return;
            }
        }

        let _ = self
            .job_scheduler
            .update_job_stage(
                &job_id,
                DubbingPipelineStage::ExtractOrGenerateTranscript,
                JobProgress {
                    percent: 50,
                    message: "Extracting audio...".into(),
                    current_step: Some("extracting_audio".into()),
                    processed_items: None,
                    total_items: None,
                },
            )
            .await;

        let target_dir = self
            .target_dir_base
            .join("auralis")
            .join("projects")
            .join(project_id.to_string())
            .join("subtitles");

        let import_use_case = ImportYoutubeSubtitlesUseCase::new(
            Arc::new(self.project_repo.clone()),
            Arc::new(self.subtitle_source.clone()),
            Arc::new(self.artifact_store.clone()),
            Arc::new(self.storage_uow.clone()),
        );

        match import_use_case
            .execute(ImportYoutubeSubtitlesRequest {
                project_id: project_id.clone(),
                target_dir,
                preferred_languages: vec!["en".to_string(), "ru".to_string(), "uk".to_string()],
                allow_auto_generated: true,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => {
                let _ = self
                    .job_scheduler
                    .fail_job(
                        &job_id,
                        "SUBTITLE_IMPORT_FAILED".into(),
                        e.to_string(),
                        false,
                    )
                    .await;
                return;
            }
        }

        // Check cancellation again
        if let Ok(Some(job)) = self.job_scheduler.get_job(&job_id).await {
            if job.status == JobStatus::Cancelled {
                return;
            }
        }

        // ExportResult stage
        let _ = self
            .job_scheduler
            .update_job_stage(
                &job_id,
                DubbingPipelineStage::ExportResult,
                JobProgress {
                    percent: 100,
                    message: "Mock stage: ExportResult".into(),
                    current_step: Some("export_result".into()),
                    processed_items: None,
                    total_items: None,
                },
            )
            .await;
        sleep(Duration::from_millis(500)).await;

        let _ = self.job_scheduler.complete_job(&job_id).await;
    }
}
