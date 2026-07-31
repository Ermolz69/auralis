#[derive(Debug, sqlx::FromRow)]
pub struct ArtifactRow {
    pub id: String,
    pub project_id: String,
    pub kind: String,
    pub location_kind: String,
    pub location_value: String,
    pub size_bytes: Option<i64>,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub ready_at: Option<String>,
}
