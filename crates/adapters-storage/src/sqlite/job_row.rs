use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct JobRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub stage: Option<String>,
    pub progress_json: String,
    pub error_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}
