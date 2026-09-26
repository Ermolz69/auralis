use adapters_storage::sqlite::SqliteProjectRepository;
use domain::project::Project;
use ports::repository::ProjectRepository;
use sqlx::SqlitePool;
use std::error::Error;
use uuid::Uuid;

pub async fn seed_run(pool: &SqlitePool) -> Result<(Project, Uuid, Uuid), Box<dyn Error>> {
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Translation events".into())?)
        .await?;
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES ('event-source', ?, 'OriginalSubtitle', 'source.srt', 'ready', 'now', 'now')")
        .bind(project.id().to_string()).execute(pool).await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES (?, ?, 'event-source', ?, 'srt', 'zh', 'ru', ?, 'now', 'now')")
        .bind(translation_id.to_string()).bind(project.id().to_string())
        .bind("a".repeat(64)).bind(run_id.to_string()).execute(pool).await?;
    sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES (?, ?, '{}', ?, 1, 0, 0, 1, ?, 'now')")
        .bind(run_id.to_string()).bind(translation_id.to_string())
        .bind("b".repeat(64)).bind("c".repeat(64)).execute(pool).await?;
    Ok((project, translation_id, run_id))
}
