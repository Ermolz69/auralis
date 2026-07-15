use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct OutboxRow {
    pub id: String,
    pub kind: String,
    pub payload_json: String,
    pub status: String,
    pub attempts: i64,
    pub next_attempt_at: String,
    pub locked_at: Option<String>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub deduplication_key: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
