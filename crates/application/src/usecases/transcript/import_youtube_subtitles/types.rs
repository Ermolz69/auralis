use domain::{job::JobId, media::SubtitleTrack, project::ProjectId, transcript::Transcript};
use ports::cancellation::CancellationToken;

pub struct ImportYoutubeSubtitlesRequest {
    pub project_id: ProjectId,
    pub preferred_languages: Vec<String>,
    pub allow_auto_generated: bool,
    pub cancellation_token: CancellationToken,
    pub job_id: JobId,
    pub selected_track: Option<SubtitleTrack>,
}

pub struct ImportYoutubeSubtitlesResponse {
    pub transcript: Transcript,
}
