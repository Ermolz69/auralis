use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub source_json: Option<String>,
    pub metadata_json: Option<String>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
    pub transcript_json: Option<String>,
    pub artifacts_json: String,
    pub created_at: String,
    pub updated_at: String,
}
