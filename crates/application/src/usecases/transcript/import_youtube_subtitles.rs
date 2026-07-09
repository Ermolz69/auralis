use std::path::PathBuf;
use std::sync::Arc;

use domain::project::ProjectId;
use domain::transcript::{Transcript, TranscriptSegment, TranscriptSegmentId};
use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;

use crate::error::ApplicationError;

pub struct ImportYoutubeSubtitlesRequest {
    pub project_id: ProjectId,
    pub target_dir: PathBuf,
    pub preferred_languages: Vec<String>,
    pub allow_auto_generated: bool,
}

pub struct ImportYoutubeSubtitlesResponse {
    pub transcript: Transcript,
}

pub struct ImportYoutubeSubtitlesUseCase {
    project_repo: Arc<dyn ProjectRepository>,
    subtitle_source: Arc<dyn SubtitleSourcePort>,
    artifact_index: Arc<dyn ArtifactIndex>,
    artifact_store: Arc<dyn ArtifactStore>,
}

impl ImportYoutubeSubtitlesUseCase {
    pub fn new(
        project_repo: Arc<dyn ProjectRepository>,
        subtitle_source: Arc<dyn SubtitleSourcePort>,
        artifact_index: Arc<dyn ArtifactIndex>,
        artifact_store: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            project_repo,
            subtitle_source,
            artifact_index,
            artifact_store,
        }
    }

    pub async fn execute(
        &self,
        request: ImportYoutubeSubtitlesRequest,
    ) -> Result<ImportYoutubeSubtitlesResponse, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no source".to_string(),
            })?;

        let subtitles = self.subtitle_source.list_subtitles(source).await?;
        if subtitles.is_empty() {
            return Err(ApplicationError::InvalidOperation {
                message: "No subtitles found".to_string(),
            });
        }

        let is_vtt = |t: &domain::media::SubtitleTrack| t.format.as_deref() == Some("vtt");

        // Pick best subtitle track
        let mut best_track = None;
        for lang in &request.preferred_languages {
            if let Some(track) = subtitles
                .iter()
                .find(|t| &t.language == lang && !t.is_auto_generated && is_vtt(t))
            {
                best_track = Some(track);
                break;
            }
        }

        if best_track.is_none() && request.allow_auto_generated {
            for lang in &request.preferred_languages {
                if let Some(track) = subtitles
                    .iter()
                    .find(|t| &t.language == lang && t.is_auto_generated && is_vtt(t))
                {
                    best_track = Some(track);
                    break;
                }
            }
        }

        if best_track.is_none() {
            // fallback to first available manual subtitle, or auto
            best_track = subtitles.iter().find(|t| !t.is_auto_generated && is_vtt(t));
            if best_track.is_none() && request.allow_auto_generated {
                best_track = subtitles.iter().find(|t| is_vtt(t));
            }
        }

        let best_track = best_track.ok_or_else(|| ApplicationError::InvalidOperation {
            message: "No suitable subtitles found".to_string(),
        })?;

        let artifact = self
            .subtitle_source
            .download_subtitle(source, best_track, &request.target_dir)
            .await?;

        let vtt_path = match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            _ => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Invalid subtitle artifact location".to_string(),
                });
            }
        };

        let vtt_content =
            std::fs::read_to_string(&vtt_path).map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to read vtt file: {}", e),
            })?;

        let transcript = parse_vtt(&vtt_content, &best_track.language)?;

        let managed_artifact = self
            .artifact_store
            .write_small_artifact(
                &request.project_id,
                domain::media::ArtifactKind::OriginalSubtitle,
                "subtitles.vtt",
                vtt_content.as_bytes(),
            )
            .await?;

        if let Err(e) = self
            .artifact_index
            .add(&request.project_id, &managed_artifact)
            .await
        {
            let _ = self.artifact_store.delete_artifact(&managed_artifact).await;
            return Err(e.into());
        }

        project.set_transcript(transcript.clone());
        self.project_repo.save(&project).await?;

        // Best effort cleanup of temp file
        let _ = std::fs::remove_file(&vtt_path);

        Ok(ImportYoutubeSubtitlesResponse { transcript })
    }
}

#[allow(clippy::collapsible_if)]
fn parse_vtt(content: &str, language: &str) -> Result<Transcript, ApplicationError> {
    let mut segments = Vec::new();
    let mut current_start = None;
    let mut current_end = None;
    let mut current_text = String::new();
    let mut index = 0;

    let flush = |start: &mut Option<u64>,
                 end: &mut Option<u64>,
                 text: &mut String,
                 segments: &mut Vec<TranscriptSegment>,
                 index: &mut u32| {
        if let (Some(s), Some(e)) = (*start, *end) {
            let t = text.trim();
            if !t.is_empty() {
                segments.push(TranscriptSegment {
                    id: TranscriptSegmentId::new(),
                    index: *index,
                    start_ms: s,
                    end_ms: e,
                    source_text: t.to_string(),
                    translated_text: None,
                    adapted_text: None,
                    speaker: None,
                    confidence: None,
                });
                *index += 1;
            }
        }
        *start = None;
        *end = None;
        text.clear();
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(
                &mut current_start,
                &mut current_end,
                &mut current_text,
                &mut segments,
                &mut index,
            );
            continue;
        }

        if line == "WEBVTT" {
            continue;
        }

        if line.contains("-->") {
            // Might have been preceded by an identifier, flush it
            current_text.clear();

            let parts: Vec<&str> = line.split("-->").collect();
            if parts.len() == 2 {
                let start_str = parts[0].split_whitespace().last().unwrap_or("");
                let end_str = parts[1].split_whitespace().next().unwrap_or("");
                if let (Some(s), Some(e)) = (parse_vtt_time(start_str), parse_vtt_time(end_str)) {
                    current_start = Some(s);
                    current_end = Some(e);
                }
            }
        } else if current_start.is_some() {
            // this is text
            // remove vtt tags like <c.color>...</c> or <v Speaker>...</v>
            let clean = remove_vtt_tags(line);
            if !current_text.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(&clean);
        }
    }

    flush(
        &mut current_start,
        &mut current_end,
        &mut current_text,
        &mut segments,
        &mut index,
    );

    Ok(Transcript {
        language: language.to_string(),
        segments,
    })
}

fn parse_vtt_time(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.split('.').collect();
    if parts.len() != 2 {
        return None;
    }
    let ms: u64 = parts[1].parse().ok()?;
    let hms: Vec<&str> = parts[0].split(':').collect();

    let (h, m, s) = match hms.len() {
        3 => (
            hms[0].parse::<u64>().ok()?,
            hms[1].parse::<u64>().ok()?,
            hms[2].parse::<u64>().ok()?,
        ),
        2 => (0, hms[0].parse::<u64>().ok()?, hms[1].parse::<u64>().ok()?),
        _ => return None,
    };

    Some(h * 3600000 + m * 60000 + s * 1000 + ms)
}

fn remove_vtt_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vtt() {
        let vtt = r#"WEBVTT

1
00:00:01.000 --> 00:00:04.000 align:start position:0%
Hello <c.colorE5E5E5>world</c>!

00:00:05.500 --> 00:00:07.000
<v Speaker>This is a test</v>
"#;
        let transcript = parse_vtt(vtt, "en").unwrap();
        assert_eq!(transcript.language, "en");
        assert_eq!(transcript.segments.len(), 2);

        let s1 = &transcript.segments[0];
        assert_eq!(s1.start_ms, 1000);
        assert_eq!(s1.end_ms, 4000);
        assert_eq!(s1.source_text, "Hello world!");

        let s2 = &transcript.segments[1];
        assert_eq!(s2.start_ms, 5500);
        assert_eq!(s2.end_ms, 7000);
        assert_eq!(s2.source_text, "This is a test");
    }
}
