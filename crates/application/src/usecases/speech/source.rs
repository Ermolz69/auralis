use super::*;
use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::speech_workflow::{SpeechCue, SpeechReview, SpeechSourceSnapshot};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechSourceRequest {
    pub kind: String,
    pub text: Option<String>,
    pub translation_id: Option<Uuid>,
    pub artifact_id: Option<domain::media::ArtifactId>,
    pub format: Option<String>,
}

impl SpeechUseCases {
    pub async fn source(
        &self,
        project_id: &ProjectId,
        request: SpeechSourceRequest,
    ) -> Result<SpeechSourceSnapshot, ApplicationError> {
        let project = self
            .projects
            .get(project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project_id.clone()))?;
        let (revision, source_id, text, cues, sha) = match request.kind.as_str() {
            "text" => {
                let text = request.text.ok_or_else(|| invalid("Enter Russian text"))?;
                let sha = hash(text.as_bytes());
                (sha.clone(), None, text, Vec::new(), sha)
            }
            "subtitles" => {
                if let Some(artifact_id) = request.artifact_id {
                    let format = match request.format.as_deref() {
                        Some("srt") => ports::translation_inspection::TranslationSourceFormat::Srt,
                        Some("vtt") => ports::translation_inspection::TranslationSourceFormat::Vtt,
                        _ => return Err(invalid("Select SRT or VTT source format").into()),
                    };
                    let inspected = self
                        .subtitles
                        .execute(super::super::translation::InspectTranslationSourceRequest {
                            project_id: project_id.clone(),
                            source_artifact_id: artifact_id.clone(),
                            format,
                        })
                        .await?;
                    let cues: Vec<_> = inspected
                        .segments
                        .iter()
                        .map(|cue| SpeechCue {
                            id: format!("cue-{}", cue.id),
                            text: cue.source_lines.join("\n"),
                            role: "Narrator".into(),
                            start_ms: cue.start_ms,
                            end_ms: cue.end_ms,
                        })
                        .collect();
                    let text = cues
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.limits()?.source(&text, cues.len())?;
                    if cues.is_empty() || text.trim().is_empty() {
                        return Err(
                            invalid("Subtitle source is empty or exceeds its limits").into()
                        );
                    }
                    return Ok(SpeechSourceSnapshot {
                        kind: format!(
                            "artifact_{}",
                            match format {
                                ports::translation_inspection::TranslationSourceFormat::Srt =>
                                    "srt",
                                ports::translation_inspection::TranslationSourceFormat::Vtt =>
                                    "vtt",
                            }
                        ),
                        source_id: Some(artifact_id.0),
                        revision: inspected.source_sha256.clone(),
                        sha256: inspected.source_sha256,
                        text,
                        cues,
                    });
                }
                let transcript = project
                    .transcript()
                    .ok_or_else(|| invalid("Project has no original transcript"))?;
                if !russian(&transcript.language) {
                    return Err(invalid("Original subtitles must be Russian").into());
                }
                let mut segments = transcript.segments.clone();
                segments.sort_by_key(|cue| cue.index);
                let cues: Vec<_> = segments
                    .iter()
                    .map(|cue| SpeechCue {
                        id: cue.id.0.to_string(),
                        text: cue.source_text.clone(),
                        role: cue.speaker.clone().unwrap_or_else(|| "Narrator".into()),
                        start_ms: cue.start_ms,
                        end_ms: cue.end_ms,
                    })
                    .collect();
                let text = cues
                    .iter()
                    .map(|cue| cue.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let sha = hash(&serde_json::to_vec(&cues).map_err(|e| invalid(e.to_string()))?);
                (project.revision().to_string(), None, text, cues, sha)
            }
            "translation" => {
                let id = request
                    .translation_id
                    .ok_or_else(|| invalid("Select translated subtitles"))?;
                let verified = self
                    .translation_review
                    .require_approved_selected(project_id.clone(), id)
                    .await?;
                if !russian(&verified.link.target_language) {
                    return Err(invalid("Selected translation must be Russian").into());
                }
                let cues: Vec<_> = verified
                    .result
                    .segments
                    .iter()
                    .map(|cue| SpeechCue {
                        id: format!("cue-{}", cue.id),
                        text: cue.translated_lines.join("\n"),
                        role: "Narrator".into(),
                        start_ms: cue.start_ms,
                        end_ms: cue.end_ms,
                    })
                    .collect();
                let text = cues
                    .iter()
                    .map(|cue| cue.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                (
                    format!("{}-{}", verified.result.result_id, verified.link.revision),
                    Some(id),
                    text,
                    cues,
                    verified.result.output_sha256,
                )
            }
            _ => return Err(invalid("Unknown speech source").into()),
        };
        self.limits()?.source(&text, cues.len())?;
        if text.trim().is_empty() || (request.kind != "text" && cues.is_empty()) {
            return Err(invalid("Speech source is empty or exceeds its limits").into());
        }
        Ok(SpeechSourceSnapshot {
            kind: request.kind,
            source_id,
            revision,
            sha256: sha,
            text,
            cues,
        })
    }

    pub async fn review(
        &self,
        project: ProjectId,
        request: SpeechSourceRequest,
        confirmed: bool,
    ) -> Result<SpeechReview, ApplicationError> {
        if request.kind != "translation" && !confirmed {
            return Err(invalid("Confirm: Текст проверен").into());
        }
        let source = self.source(&project, request).await?;
        let review = SpeechReview {
            original_source: None,
            decision_id: Uuid::new_v4(),
            project_id: project,
            source,
        };
        self.store.save_review(&review).await?;
        Ok(review)
    }

    pub(super) async fn check_source(&self, review: &SpeechReview) -> Result<(), ApplicationError> {
        let source = review.original_source.as_ref().unwrap_or(&review.source);
        let artifact = source.kind.starts_with("artifact_");
        let current = self
            .source(
                &review.project_id,
                SpeechSourceRequest {
                    kind: if artifact {
                        "subtitles".into()
                    } else {
                        source.kind.clone()
                    },
                    translation_id: if artifact { None } else { source.source_id },
                    text: Some(source.text.clone()),
                    artifact_id: if artifact {
                        source.source_id.map(domain::media::ArtifactId)
                    } else {
                        None
                    },
                    format: if artifact {
                        source.kind.strip_prefix("artifact_").map(str::to_owned)
                    } else {
                        None
                    },
                },
            )
            .await?;
        if &current != source {
            return Err(conflict(
                "Text or selected translation changed; review it again and prepare a new run",
            )
            .into());
        }
        Ok(())
    }
}

fn russian(language: &str) -> bool {
    matches!(
        language.to_lowercase().as_str(),
        "ru" | "rus" | "russian" | "русский" | "ru-ru"
    )
}
