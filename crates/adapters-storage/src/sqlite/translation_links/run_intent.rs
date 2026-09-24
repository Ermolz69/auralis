use super::validation;
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqliteExecutor};
use uuid::Uuid;

const MAX_PROFILE_BYTES: usize = 65_536;
const MAX_TARGET_SEGMENTS: u32 = 64;
const MAX_CONTEXT_SEGMENTS: u32 = 8;

#[derive(FromRow)]
struct RunIntentRow {
    run_id: String,
    translation_id: String,
    profile_manifest_json: String,
    profile_sha256: String,
    max_target_segments: i64,
    context_before_segments: i64,
    context_after_segments: i64,
    parser_version: i64,
    policy_fingerprint: String,
    created_at: String,
}

pub(super) fn validate(run: &TranslationRunIntent) -> Result<(), PortError> {
    validation::uuid(run.run_id, "run ID")?;
    validation::uuid(run.translation_id, "translation ID")?;
    if run.profile_manifest_json.is_empty()
        || run.profile_manifest_json.len() > MAX_PROFILE_BYTES
        || !serde_json::from_str::<serde_json::Value>(&run.profile_manifest_json)
            .is_ok_and(|value| value.is_object())
        || run.max_target_segments == 0
        || run.max_target_segments > MAX_TARGET_SEGMENTS
        || run.context_before_segments > MAX_CONTEXT_SEGMENTS
        || run.context_after_segments > MAX_CONTEXT_SEGMENTS
        || run.parser_version == 0
        || !is_hash(&run.profile_sha256)
        || !is_hash(&run.policy_fingerprint)
        || format!("{:x}", Sha256::digest(run.profile_manifest_json.as_bytes()))
            != run.profile_sha256
    {
        return Err(PortError::InvalidSource {
            message: "invalid frozen translation run intent".into(),
        });
    }
    Ok(())
}

pub(super) async fn ensure<'e, E>(executor: E, run: &TranslationRunIntent) -> Result<(), PortError>
where
    E: SqliteExecutor<'e>,
{
    validate(run)?;
    sqlx::query(
        "INSERT INTO translation_run_intents (
            run_id, translation_id, profile_manifest_json, profile_sha256,
            max_target_segments, context_before_segments, context_after_segments,
            parser_version, policy_fingerprint, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(run_id) DO NOTHING",
    )
    .bind(run.run_id.to_string())
    .bind(run.translation_id.to_string())
    .bind(&run.profile_manifest_json)
    .bind(&run.profile_sha256)
    .bind(i64::from(run.max_target_segments))
    .bind(i64::from(run.context_before_segments))
    .bind(i64::from(run.context_after_segments))
    .bind(i64::from(run.parser_version))
    .bind(&run.policy_fingerprint)
    .bind(run.created_at.to_rfc3339())
    .execute(executor)
    .await
    .map_err(|error| {
        super::super::helpers::map_sqlite_error("ensure_translation_run_intent", error)
    })?;
    Ok(())
}

pub(super) async fn load<'e, E>(
    executor: E,
    run_id: Uuid,
) -> Result<Option<TranslationRunIntent>, PortError>
where
    E: SqliteExecutor<'e>,
{
    sqlx::query_as::<_, RunIntentRow>(
        "SELECT run_id, translation_id, profile_manifest_json, profile_sha256,
            max_target_segments, context_before_segments, context_after_segments,
            parser_version, policy_fingerprint, created_at
         FROM translation_run_intents WHERE run_id = ?",
    )
    .bind(run_id.to_string())
    .fetch_optional(executor)
    .await
    .map_err(|error| super::super::helpers::map_sqlite_error("read_translation_run_intent", error))?
    .map(RunIntentRow::decode)
    .transpose()
}

fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_ascii_lowercase()
}

impl RunIntentRow {
    fn decode(self) -> Result<TranslationRunIntent, PortError> {
        let invalid = |field: &str| PortError::InvalidStoredData {
            entity_type: "TranslationRunIntent".into(),
            entity_id: self.run_id.clone(),
            field: field.into(),
            message: "invalid stored value".into(),
        };
        let intent = TranslationRunIntent {
            run_id: Uuid::parse_str(&self.run_id).map_err(|_| invalid("run_id"))?,
            translation_id: Uuid::parse_str(&self.translation_id)
                .map_err(|_| invalid("translation_id"))?,
            profile_manifest_json: self.profile_manifest_json,
            profile_sha256: self.profile_sha256,
            max_target_segments: u32::try_from(self.max_target_segments)
                .map_err(|_| invalid("max_target_segments"))?,
            context_before_segments: u32::try_from(self.context_before_segments)
                .map_err(|_| invalid("context_before_segments"))?,
            context_after_segments: u32::try_from(self.context_after_segments)
                .map_err(|_| invalid("context_after_segments"))?,
            parser_version: u32::try_from(self.parser_version)
                .map_err(|_| invalid("parser_version"))?,
            policy_fingerprint: self.policy_fingerprint,
            created_at: self.created_at.parse().map_err(|_| invalid("created_at"))?,
        };
        validate(&intent).map_err(|_| invalid("frozen_run_intent"))?;
        Ok(intent)
    }
}
