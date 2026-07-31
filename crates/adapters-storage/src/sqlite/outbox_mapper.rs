use domain::outbox::{OutboxMessage, OutboxMessageId, OutboxMessageStatus, OutboxPayload};
use ports::error::PortError;
use std::str::FromStr;

use super::outbox_row::OutboxRow;

pub fn row_to_outbox_message(row: OutboxRow) -> Result<OutboxMessage, PortError> {
    let id = parse_outbox_message_id(&row.id)?;

    let payload: OutboxPayload =
        serde_json::from_str(&row.payload_json).map_err(|e| PortError::InvalidStoredData {
            entity_type: "outbox".to_string(),
            entity_id: row.id.clone(),
            field: "payload".to_string(),
            message: format!("Failed to parse outbox payload: {}", e),
        })?;

    let status = match row.status.as_str() {
        "pending" => OutboxMessageStatus::Pending,
        "processing" => OutboxMessageStatus::Processing,
        "done" => OutboxMessageStatus::Done,
        "failed" => OutboxMessageStatus::Failed,
        "dead" => OutboxMessageStatus::Dead,
        _ => {
            return Err(PortError::InvalidStoredData {
                entity_type: "outbox".to_string(),
                entity_id: row.id.clone(),
                field: "status".to_string(),
                message: format!("Unknown outbox status: {}", row.status),
            });
        }
    };

    let next_attempt_at = parse_datetime(&row.next_attempt_at, &row.id, "next_attempt_at")?;
    let locked_at = row
        .locked_at
        .map(|s| parse_datetime(&s, &row.id, "locked_at"))
        .transpose()?;
    let created_at = parse_datetime(&row.created_at, &row.id, "created_at")?;
    let updated_at = parse_datetime(&row.updated_at, &row.id, "updated_at")?;

    let aggregate_type: Option<String> = row.aggregate_type.clone();
    let aggregate_id: Option<String> = row.aggregate_id.clone();

    Ok(OutboxMessage {
        id,
        payload,
        status,
        attempts: row.attempts as i32,
        next_attempt_at,
        locked_at,
        locked_by: row.locked_by,
        last_error: row.last_error,
        deduplication_key: row.deduplication_key,
        created_at,
        updated_at,
        aggregate_type,
        aggregate_id,
    })
}

fn parse_outbox_message_id(value: &str) -> Result<OutboxMessageId, PortError> {
    OutboxMessageId::from_str(value).map_err(|e| PortError::InvalidStoredData {
        entity_type: "outbox".to_string(),
        entity_id: value.to_string(),
        field: "id".to_string(),
        message: format!("Failed to parse outbox id: {}", e),
    })
}

fn parse_datetime(
    value: &str,
    outbox_id: &str,
    field: &'static str,
) -> Result<chrono::DateTime<chrono::Utc>, PortError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|e| PortError::InvalidStoredData {
            entity_type: "outbox".to_string(),
            entity_id: outbox_id.to_string(),
            field: field.to_string(),
            message: format!("Failed to parse datetime: {}", e),
        })
        .map(|dt| dt.with_timezone(&chrono::Utc))
}
