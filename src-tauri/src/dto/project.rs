use crate::dto::media::{MediaMetadataDto, MediaSourceDto};
use domain::project::Project;
use jobs::job::Job;
use serde::Serialize;

#[derive(Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub source: Option<MediaSourceDto>,
    pub metadata: Option<MediaMetadataDto>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Project> for ProjectDto {
    fn from(p: &Project) -> Self {
        Self {
            id: p.id().to_string(),
            title: p.title().to_string(),
            status: format!("{:?}", p.status()),
            source: p.source().map(Into::into),
            metadata: p.metadata().map(Into::into),
            created_at: p.created_at().to_rfc3339(),
            updated_at: p.updated_at().to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
pub struct CreateProjectResponse {
    pub project: ProjectDto,
    pub job: Job,
}

#[derive(Serialize)]
pub struct TranscriptDto {
    pub language: String,
    pub segments: Vec<TranscriptSegmentDto>,
}

impl From<&domain::transcript::Transcript> for TranscriptDto {
    fn from(t: &domain::transcript::Transcript) -> Self {
        Self {
            language: t.language.clone(),
            segments: t.segments.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct TranscriptSegmentDto {
    pub id: String,
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_text: String,
}

impl From<&domain::transcript::TranscriptSegment> for TranscriptSegmentDto {
    fn from(s: &domain::transcript::TranscriptSegment) -> Self {
        Self {
            id: s.id.0.to_string(),
            index: s.index,
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            source_text: s.source_text.clone(),
        }
    }
}
