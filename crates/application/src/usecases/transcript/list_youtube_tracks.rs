use crate::error::ApplicationError;
use domain::{media::SubtitleTrack, project::ProjectId};
use ports::{repository::ProjectRepository, source::SubtitleSourcePort};

pub struct ListYoutubeSubtitleTracksRequest {
    pub project_id: ProjectId,
}

pub struct ListYoutubeSubtitleTracksResponse {
    pub tracks: Vec<SubtitleTrack>,
}

pub struct ListYoutubeSubtitleTracksUseCase<R: ProjectRepository, S: SubtitleSourcePort> {
    project_repo: R,
    subtitle_source: S,
}

impl<R: ProjectRepository, S: SubtitleSourcePort> ListYoutubeSubtitleTracksUseCase<R, S> {
    pub fn new(project_repo: R, subtitle_source: S) -> Self {
        Self {
            project_repo,
            subtitle_source,
        }
    }

    pub async fn execute(
        &self,
        request: ListYoutubeSubtitleTracksRequest,
    ) -> Result<ListYoutubeSubtitleTracksResponse, ApplicationError> {
        let project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;
        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no source".into(),
            })?;

        let mut tracks: Vec<_> = self
            .subtitle_source
            .list_subtitles(source)
            .await?
            .into_iter()
            .filter(|track| track.format.as_deref() == Some("vtt"))
            .collect();

        tracks.sort_by(|left, right| {
            track_priority(left)
                .cmp(&track_priority(right))
                .then_with(|| left.language.cmp(&right.language))
                .then_with(|| left.is_auto_generated.cmp(&right.is_auto_generated))
        });

        Ok(ListYoutubeSubtitleTracksResponse { tracks })
    }
}

fn track_priority(track: &SubtitleTrack) -> u8 {
    if track
        .language
        .split(['-', '_'])
        .next()
        .is_some_and(|language| language.eq_ignore_ascii_case("ru"))
    {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_tracks_have_highest_priority() {
        let russian = SubtitleTrack {
            id: "ru-manual-vtt".into(),
            language: "ru".into(),
            label: None,
            format: Some("vtt".into()),
            is_auto_generated: false,
        };
        let english = SubtitleTrack {
            id: "en-manual-vtt".into(),
            language: "en".into(),
            label: None,
            format: Some("vtt".into()),
            is_auto_generated: false,
        };

        assert!(track_priority(&russian) < track_priority(&english));
    }
}
