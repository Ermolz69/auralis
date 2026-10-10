use async_trait::async_trait;
use chrono::Utc;
use domain::job::{Job, JobKind, JobStatus};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::speech_run::{ActiveSpeechAttempt, SpeechCheckpoint, SpeechRunIntent, SpeechRunStore};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use super::helpers::map_sqlite_error;
use super::job_mapper::{job_to_row_values, row_to_job};
use super::job_row::JobRow;

pub struct SqliteSpeechRunStore {
    pool: SqlitePool,
}

impl SqliteSpeechRunStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

mod operations;

fn decode_active(row: sqlx::sqlite::SqliteRow) -> Result<ActiveSpeechAttempt, PortError> {
    let job = row_to_job(JobRow::from_row(&row).map_err(decode_error)?)?;
    let raw_run: String = row.try_get("host_run_id").map_err(decode_error)?;
    let raw_attempt: String = row.try_get("host_attempt_id").map_err(decode_error)?;
    Ok(ActiveSpeechAttempt {
        job,
        run_id: Uuid::parse_str(&raw_run)
            .map_err(|error| invalid_stored(Uuid::nil(), "run_id", error.to_string()))?,
        attempt_id: Uuid::parse_str(&raw_attempt)
            .map_err(|error| invalid_stored(Uuid::nil(), "attempt_id", error.to_string()))?,
    })
}

fn validate_intent(intent: &SpeechRunIntent) -> Result<(), PortError> {
    if intent.plan_json.is_empty()
        || intent.plan_json.len() > 16_777_216
        || !serde_json::from_str::<serde_json::Value>(&intent.plan_json)
            .is_ok_and(|value| value.is_object())
        || !valid_hash(&intent.plan_sha256)
        || format!("{:x}", Sha256::digest(intent.plan_json.as_bytes())) != intent.plan_sha256
        || !portable_id(&intent.profile_id)
        || !valid_hash(&intent.profile_sha256)
        || intent.expected_utterances == 0
        || intent.expected_utterances > 512
        || intent.expected_chunks < intent.expected_utterances
        || intent.expected_chunks > 4096
    {
        Err(invalid_input("invalid immutable speech run intent"))
    } else {
        Ok(())
    }
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn portable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn revision(value: u64) -> Result<i64, PortError> {
    revision_value(value)
}

fn revision_value(value: u64) -> Result<i64, PortError> {
    i64::try_from(value)
        .map_err(|_| invalid_input("speech revision or duration exceeds SQLite range"))
}

fn invalid_input(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "speech run".into(),
        message: message.into(),
    }
}

fn invalid_stored(run_id: Uuid, field: &str, message: String) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "speech run".into(),
        entity_id: run_id.to_string(),
        field: field.into(),
        message,
    }
}

fn decode_error(error: sqlx::Error) -> PortError {
    map_sqlite_error("decode_speech_run", error)
}

fn conflict_or_storage(operation: &'static str, error: sqlx::Error) -> PortError {
    if let sqlx::Error::Database(ref database_error) = error
        && database_error.is_unique_violation()
    {
        conflict("speech run, attempt, job or checkpoint identity already exists")
    } else {
        map_sqlite_error(operation, error)
    }
}
