use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::media::{MediaMetadata, MediaSource};
use crate::transcript::Transcript;

use super::{LanguageCode, ProjectId, ProjectStatus};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSnapshot {
    pub id: ProjectId,
    pub title: String,
    pub status: ProjectStatus,
    pub source: Option<MediaSource>,
    pub metadata: Option<MediaMetadata>,
    pub source_language: Option<LanguageCode>,
    pub target_language: Option<LanguageCode>,
    pub transcript: Option<Transcript>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
