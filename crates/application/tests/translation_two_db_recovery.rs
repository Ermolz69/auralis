use adapters_storage::sqlite::{
    SqliteJobRepository, SqliteProjectRepository, SqliteTranslationHostJobStore,
    SqliteTranslationLinkStore, connect_sqlite,
};
use adapters_translate::{TranslateRunControl, TranslateRunRecovery};
use application::usecases::translation::{
    RecoverTranslationHostJobsUseCase, StartTranslationJobUseCase, TranslationRunRequest,
};
use auralis_translation::{
    LanguageCode, LanguagePair, RunId, RunState, SegmentId, SourceHash, TargetSegment,
    TranslationId,
};
use auralis_translation_sqlite::{
    CheckpointSpec, RunSpec, SqliteConfig, TranslateDb, TranslationSpec,
};
use chrono::Utc;
use domain::job::{Job, JobKind, JobStatus};
use domain::media::ArtifactId;
use domain::project::{Project, ProjectId};
use ports::repository::{JobRepository, ProjectRepository};
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_links::TranslationLinkStore;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use sqlx::SqlitePool;
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

async fn seed_link(pool: &SqlitePool) -> Result<(ProjectId, Uuid, Uuid), Box<dyn Error>> {
    let project = SqliteProjectRepository::new(pool.clone())
        .create(Project::new("Recovery".into())?)
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

fn checkpoint(
    run_id: RunId,
    block_index: u32,
    text: &str,
) -> Result<CheckpointSpec, Box<dyn Error>> {
    Ok(CheckpointSpec {
        run_id,
        block_index,
        input_fingerprint: SourceHash::digest(format!("block-{block_index}").as_bytes()),
        accepted: vec![TargetSegment {
            id: SegmentId::new(block_index + 1).ok_or("invalid segment ID")?,
            lines: vec![text.into()],
        }],
        diagnostics_json: "[]".into(),
        attempt_count: 1,
    })
}

#[tokio::test]
async fn restart_reconciles_host_and_translate_databases_without_losing_checkpoint()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let host_path = directory.path().join("auralis.sqlite");
    let translate_path = directory.path().join("translate.sqlite");
    let pool = connect_sqlite(&host_path).await?;
    let (project_id, translation_id, run_uuid) = seed_link(&pool).await?;
    let links = SqliteTranslationLinkStore::new(pool.clone());
    let link = links.get(translation_id).await?.ok_or("missing link")?;
    let run_id = RunId::new(run_uuid).ok_or("invalid run ID")?;
    let source_hash = SourceHash::parse_hex(&link.source_sha256).ok_or("invalid source hash")?;
    let run = RunSpec {
        run_id,
        translation_id: TranslationId::new(translation_id).ok_or("invalid translation ID")?,
        source_hash,
        profile_fingerprint: "frozen-profile".into(),
        parser_version: 1,
        policy_fingerprint: "frozen-policy".into(),
        glossary_revision: None,
        blocks: vec![
            vec![SegmentId::new(1).ok_or("invalid first segment")?],
            vec![SegmentId::new(2).ok_or("invalid second segment")?],
        ],
    };
    let mut translate = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    translate.ensure_translation(&TranslationSpec {
        translation_id: run.translation_id,
        project_id: Some(project_id.to_string()),
        source_artifact_id: Some(link.source_artifact_id.to_string()),
        source_locator: None,
        source_hash,
        source_format: "srt".into(),
        language_pair: LanguagePair::new(LanguageCode::Chinese, LanguageCode::Russian)?,
    })?;
    translate.ensure_run(&run)?;
    let jobs = SqliteTranslationHostJobStore::new(pool.clone());
    let pending = jobs
        .create_pending(
            Job::new(
                project_id.clone(),
                "Initial attempt".into(),
                JobKind::Translation,
            ),
            translation_id,
            run_uuid,
        )
        .await?;
    let mut running = pending.clone();
    running.start()?;
    jobs.start(&running, pending.revision()).await?;
    translate.begin_attempt(&run, Some(&pending.id().to_string()))?;
    translate.commit_checkpoint(&checkpoint(run_id, 0, "Первый блок.")?)?;
    drop(translate);
    pool.close().await;

    let restarted_pool = connect_sqlite(&host_path).await?;
    let restarted_jobs = Arc::new(SqliteTranslationHostJobStore::new(restarted_pool.clone()));
    let recovery = RecoverTranslationHostJobsUseCase::new(
        restarted_jobs.clone(),
        Arc::new(TranslateRunRecovery::new(translate_path.clone())),
    );
    let outcomes = recovery.execute().await?;
    assert_eq!(outcomes.len(), 1);
    assert!(outcomes[0].outcome.is_ok());
    assert!(recovery.execute().await?.is_empty());
    assert!(restarted_jobs.active_for_run(run_uuid).await?.is_none());
    let old_job = SqliteJobRepository::new(restarted_pool.clone())
        .get(pending.id())
        .await?
        .ok_or("missing recovered job")?;
    assert_eq!(old_job.status(), &JobStatus::Cancelled);
    assert_eq!(old_job.progress().processed_items, Some(1));
    assert_eq!(old_job.progress().total_items, Some(2));

    let restarted_links = Arc::new(SqliteTranslationLinkStore::new(restarted_pool));
    let retained = restarted_links
        .get(translation_id)
        .await?
        .ok_or("missing retained project link")?;
    assert_eq!(retained.active_run_id, Some(run_uuid));
    assert_eq!(retained.selected_result_id, None);
    assert_eq!(retained.output_artifact_id, None);
    let control = Arc::new(TranslateRunControl::new(translate_path.clone()));
    let snapshot = control.snapshot(&retained).await?;
    assert_eq!(snapshot.phase, TranslationRunPhase::Paused);
    assert_eq!((snapshot.saved_blocks, snapshot.total_blocks), (1, 2));
    let resumed = StartTranslationJobUseCase::new(restarted_links, control, restarted_jobs.clone())
        .execute(TranslationRunRequest {
            project_id,
            translation_id,
        })
        .await?;
    assert_ne!(resumed.id(), pending.id());
    assert_eq!(resumed.status(), &JobStatus::Pending);

    let mut translate = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    assert_eq!(translate.run_state(run_id)?, RunState::Paused);
    assert_eq!(translate.checkpoints(run_id)?.len(), 1);
    translate.begin_attempt(&run, Some(&resumed.id().to_string()))?;
    translate.commit_checkpoint(&checkpoint(run_id, 1, "Второй блок.")?)?;
    assert_eq!(translate.checkpoints(run_id)?.len(), 2);
    Ok(())
}
