#[derive(sqlx::FromRow)]
pub(crate) struct AssemblyIntentRow {
    pub assembly_id: String,
    pub project_id: String,
    pub source_run_id: String,
    pub source_attempt_id: String,
    pub plan_json: String,
    pub plan_sha256: String,
}
