#[derive(Debug, sqlx::FromRow)]
pub struct ArtifactRow {
    pub id: String,
    pub project_id: String,
    pub kind: String,
    pub location_kind: String,
    pub location_value: String,
    pub size_bytes: Option<i64>,
    pub created_at: String,
}
