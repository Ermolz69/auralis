use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use adapters_translate::TranslateProjectCleanup;
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use auralis_translation::{LanguageCode, LanguagePair, SourceHash, TranslationId};
use auralis_translation_sqlite::{SqliteConfig, TranslateDb, TranslationSpec};
use domain::media::ArtifactId;
use domain::project::Project;
use ports::repository::ProjectRepository;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected event")
    }

    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        panic!("unexpected event")
    }
}

#[tokio::test]
async fn project_deletion_retries_translate_cleanup_after_database_recovery()
-> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("auralis.sqlite")).await?;
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Delete translation".into())?)
        .await?;
    let source_id = ArtifactId::new();
    let translation_uuid = Uuid::new_v4();
    let translation_id = TranslationId::new(translation_uuid).ok_or("invalid translation ID")?;
    let source_hash = SourceHash::digest(b"immutable original");
    let now = domain::chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', 'source.srt', 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO project_translations (translation_id, project_id, source_artifact_id, source_sha256, source_format, source_language, target_language, active_run_id, created_at, updated_at) VALUES (?, ?, ?, ?, 'srt', 'zh', 'ru', ?, ?, ?)")
        .bind(translation_uuid.to_string())
        .bind(project.id().to_string())
        .bind(source_id.to_string())
        .bind(source_hash.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await?;

    let translate_path = root.path().join("translate.sqlite");
    let spec = TranslationSpec {
        translation_id,
        project_id: Some(project.id().to_string()),
        source_artifact_id: Some(source_id.to_string()),
        source_locator: None,
        source_hash,
        source_format: "srt".into(),
        language_pair: LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian)?,
    };
    TranslateDb::open(&translate_path, SqliteConfig::default())?.ensure_translation(&spec)?;

    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    uow.commit_project_delete(CommitProjectDelete {
        project_id: project.id().clone(),
    })
    .await?;
    let deleted_links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_translations")
        .fetch_one(&pool)
        .await?;
    assert_eq!(deleted_links, 0);
    assert!(
        TranslateDb::open(&translate_path, SqliteConfig::default())?
            .translation(translation_id)
            .is_ok()
    );

    let config = OutboxMaintenanceConfig::try_default()?;
    let worker = |database_path| {
        OutboxWorker::new(
            SqliteOutboxRepository::new(pool.clone()),
            Arc::new(LocalArtifactStore::new(root.path().join("artifacts"))),
            Arc::new(SqliteArtifactIndex::new(pool.clone())),
            SqliteStorageUnitOfWork::new(pool.clone()),
            Arc::new(NoEvents),
            Arc::new(LocalTempWorkspace::new(root.path())),
            config.clone(),
        )
        .with_translation_cleanup(Arc::new(TranslateProjectCleanup::new(database_path)))
    };
    let unavailable = worker(root.path().join("missing.sqlite"));
    let first = unavailable.process_pending_messages(10).await?;
    assert_eq!(first.retry_scheduled, 1);
    let (status, attempts): (String, i64) = sqlx::query_as(
        "SELECT status, attempts FROM outbox_messages WHERE kind = 'delete_project_translation'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "failed");
    assert_eq!(attempts, 1);
    assert!(
        TranslateDb::open(&translate_path, SqliteConfig::default())?
            .translation(translation_id)
            .is_ok()
    );

    sqlx::query("UPDATE outbox_messages SET attempts = 4, next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE kind = 'delete_project_translation'")
        .execute(&pool)
        .await?;
    let fifth = unavailable.process_pending_messages(10).await?;
    assert_eq!(fifth.retry_scheduled, 1);
    let (status, attempts): (String, i64) = sqlx::query_as(
        "SELECT status, attempts FROM outbox_messages WHERE kind = 'delete_project_translation'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "failed");
    assert_eq!(attempts, 5);

    sqlx::query("UPDATE outbox_messages SET next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE kind = 'delete_project_translation'")
        .execute(&pool)
        .await?;
    let recovered = worker(translate_path.clone());
    let final_report = recovered.process_pending_messages(10).await?;
    assert_eq!(final_report.completed, 1);
    let status: String = sqlx::query_scalar(
        "SELECT status FROM outbox_messages WHERE kind = 'delete_project_translation'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(status, "done");
    let mut translate = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    assert!(translate.translation(translation_id).is_err());
    assert!(translate.ensure_translation(&spec).is_err());
    Ok(())
}
