use crate::dto::media::{MediaMetadataDto, MediaSourceDto};
use domain::project::{Project, ProjectStatus};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
            status: match p.status() {
                ProjectStatus::Draft => "draft",
                ProjectStatus::SourceImported => "source_imported",
                ProjectStatus::ReadyForProcessing => "ready_for_processing",
                ProjectStatus::Processing => "processing",
                ProjectStatus::Completed => "completed",
                ProjectStatus::Failed => "failed",
                ProjectStatus::Cancelled => "cancelled",
            }
            .to_string(),
            source: p.source().map(Into::into),
            metadata: p.metadata().map(Into::into),
            created_at: p.created_at().to_rfc3339(),
            updated_at: p.updated_at().to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResponse {
    pub project: ProjectDto,
    pub job: ports::job_scheduler::ScheduledJob,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use domain::project::Project;

    #[test]
    fn test_empty_project_serialization() {
        let project = Project::new("Empty Test".to_string());
        let dto: ProjectDto = (&project).into();

        let json = serde_json::to_string(&dto).unwrap();

        // Assert that source and metadata are explicitly null
        assert!(json.contains(r#""source":null"#));
        assert!(json.contains(r#""metadata":null"#));

        // Assert the exact structure to avoid regressions
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("source").unwrap().is_null());
        assert!(parsed.get("metadata").unwrap().is_null());
    }
}
