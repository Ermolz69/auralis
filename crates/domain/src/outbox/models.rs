use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainError;
use crate::media::ArtifactId;
use crate::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutboxMessageId(pub Uuid);

impl OutboxMessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OutboxMessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for OutboxMessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OutboxMessageId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s)
            .map(OutboxMessageId)
            .map_err(|_| DomainError::ValidationError(format!("Invalid OutboxMessageId: {}", s)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxMessageStatus {
    Pending,
    Processing,
    Done,
    Failed,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboxPayload {
    FinalizeStagedArtifact {
        artifact_id: ArtifactId,
        staging_key: String,
        final_key: String,
    },
    DeleteStorageKey {
        storage_key: String,
    },
    DeleteProjectArtifactDir {
        project_id: ProjectId,
    },
    DeleteTempPath {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessage {
    pub id: OutboxMessageId,
    pub payload: OutboxPayload,
    pub status: OutboxMessageStatus,
    pub attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OutboxMessage {
    pub fn new(payload: OutboxPayload) -> Self {
        let now = Utc::now();
        Self {
            id: OutboxMessageId::new(),
            payload,
            status: OutboxMessageStatus::Pending,
            attempts: 0,
            next_attempt_at: now,
            locked_at: None,
            locked_by: None,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
}
