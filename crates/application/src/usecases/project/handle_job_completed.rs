use crate::usecases::transcript::import_youtube_subtitles::{
    ImportYoutubeSubtitlesRequest, ImportYoutubeSubtitlesUseCase,
};
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use std::str::FromStr;
use std::sync::Arc;

pub struct HandleJobCompletedRequest {
    pub job_id: String,
    pub project_id: String,
    pub is_success: bool,
    pub target_dir_base: std::path::PathBuf,
}

pub struct HandleJobCompletedResult {
    pub transcript_ready: bool,
}

pub struct HandleJobCompletedUseCase<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
> {
    project_repo: R,
    video_source: V,
}

impl<R: ProjectRepository + Clone + 'static, V: SubtitleSourcePort + Clone + 'static>
    HandleJobCompletedUseCase<R, V>
{
    pub fn new(project_repo: R, video_source: V) -> Self {
        Self {
            project_repo,
            video_source,
        }
    }

    pub async fn execute(
        &self,
        req: HandleJobCompletedRequest,
    ) -> Result<HandleJobCompletedResult, String> {
        let pid =
            domain::project::ProjectId::from_str(&req.project_id).map_err(|e| e.to_string())?;

        let mut project = self
            .project_repo
            .get(&pid)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Project not found".to_string())?;

        let mut transcript_ready = false;

        if req.is_success {
            let is_youtube = matches!(
                project.source(),
                Some(domain::media::MediaSource::YoutubeUrl { .. })
            );

            if is_youtube {
                let target_dir = req
                    .target_dir_base
                    .join("auralis")
                    .join("projects")
                    .join(&req.project_id)
                    .join("subtitles");

                let import_use_case = ImportYoutubeSubtitlesUseCase::new(
                    Arc::new(self.project_repo.clone()),
                    Arc::new(self.video_source.clone()),
                );

                match import_use_case
                    .execute(ImportYoutubeSubtitlesRequest {
                        project_id: pid.clone(),
                        target_dir,
                        preferred_languages: vec![
                            "en".to_string(),
                            "ru".to_string(),
                            "uk".to_string(),
                        ],
                        allow_auto_generated: true,
                    })
                    .await
                {
                    Ok(_) => {
                        // Re-fetch project to ensure we have the updated version with transcript
                        if let Ok(Some(updated_project)) = self.project_repo.get(&pid).await {
                            project = updated_project;
                        }
                        transcript_ready = true;
                        let _ = project.mark_completed();
                    }
                    Err(_) => {
                        let _ = project.mark_failed();
                    }
                }
            } else {
                let _ = project.mark_completed();
            }
        } else {
            let _ = project.mark_failed();
        }

        let _ = self.project_repo.save(&project).await;

        Ok(HandleJobCompletedResult { transcript_ready })
    }
}
