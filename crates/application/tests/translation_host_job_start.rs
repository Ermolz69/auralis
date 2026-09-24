use adapters_storage::sqlite::{
    SqliteProjectRepository, SqliteTranslationHostJobStore, SqliteTranslationLinkStore,
    connect_sqlite,
};
use application::usecases::translation::{StartTranslationJobUseCase, TranslationRunRequest};
use async_trait::async_trait;
use auralis_translation::SourceHash;
use chrono::Utc;
use domain::job::{JobKind, JobStatus};
use domain::media::ArtifactId;
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::translation_links::TranslationLink;
use ports::translation_run_control::{
    TranslationRunControlPort, TranslationRunPhase, TranslationRunSnapshot,
};
use sqlx::SqlitePool;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

struct FixedRunControl {
    run_id: Uuid,
    phase: TranslationRunPhase,
}

#[async_trait]
impl TranslationRunControlPort for FixedRunControl {
    async fn snapshot(&self, _link: &TranslationLink) -> Result<TranslationRunSnapshot, PortError> {
        Ok(TranslationRunSnapshot {
            run_id: self.run_id,
            phase: self.phase,
            pause_requested: false,
            saved_blocks: 0,
            total_blocks: 1,
            result_id: None,
            needs_review: None,
        })
    }

    async fn request_pause(&self, _link: &TranslationLink) -> Result<(), PortError> {
        Ok(())
    }
}

async fn seed_run(pool: &SqlitePool) -> Result<(ProjectId, Uuid, Uuid), Box<dyn Error>> {
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Translation".into())?)
        .await?;
    let source_id = ArtifactId::new();
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES (?, ?, ?, ?, 'srt', 'zh', 'ru', ?, ?, ?)")
        .bind(translation_id.to_string())
        .bind(project.id().to_string())
        .bind(source_id.to_string())
        .bind("a".repeat(64))
        .bind(run_id.to_string())
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO translation_run_intents (run_id, translation_id, profile_manifest_json, profile_sha256, max_target_segments, context_before_segments, context_after_segments, parser_version, policy_fingerprint, created_at) VALUES (?, ?, '{}', ?, 1, 0, 0, 1, ?, ?)")
        .bind(run_id.to_string())
        .bind(translation_id.to_string())
        .bind(SourceHash::digest(b"{}").to_string())
        .bind("b".repeat(64))
        .bind(&now)
        .execute(pool)
        .await?;
    Ok((project.id().clone(), translation_id, run_id))
}

fn use_case(
    pool: &SqlitePool,
    run_id: Uuid,
    phase: TranslationRunPhase,
) -> StartTranslationJobUseCase {
    StartTranslationJobUseCase::new(
        Arc::new(SqliteTranslationLinkStore::new(pool.clone())),
        Arc::new(FixedRunControl { run_id, phase }),
        Arc::new(SqliteTranslationHostJobStore::new(pool.clone())),
    )
}

#[tokio::test]
async fn starting_twice_returns_one_project_scoped_pending_job() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project_id, translation_id, run_id) = seed_run(&pool).await?;
    let start = use_case(&pool, run_id, TranslationRunPhase::Requested);
    let request = || TranslationRunRequest {
        project_id: project_id.clone(),
        translation_id,
    };
    let first = start.execute(request()).await?;
    let second = start.execute(request()).await?;
    assert_eq!(first.id(), second.id());
    let validated = use_case(&pool, run_id, TranslationRunPhase::Validated);
    assert!(validated.execute(request()).await.is_err());
    assert_eq!(first.kind(), &JobKind::Translation);
    assert_eq!(first.status(), &JobStatus::Pending);
    assert!(first.stage().is_none());
    assert!(
        start
            .execute(TranslationRunRequest {
                project_id: ProjectId::new(),
                translation_id,
            })
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[tokio::test]
async fn running_or_validated_translate_run_cannot_create_a_new_job() -> Result<(), Box<dyn Error>>
{
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project_id, translation_id, run_id) = seed_run(&pool).await?;
    for phase in [TranslationRunPhase::Running, TranslationRunPhase::Validated] {
        let start = use_case(&pool, run_id, phase);
        assert!(
            start
                .execute(TranslationRunRequest {
                    project_id: project_id.clone(),
                    translation_id,
                })
                .await
                .is_err()
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}
