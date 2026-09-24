use adapters_storage::{
    local::LocalArtifactStore,
    sqlite::{
        SqliteArtifactIndex, SqliteProjectRepository, SqliteTranslationLinkStore, connect_sqlite,
    },
};
use adapters_translate::{FormatInspector, TranslateRunRegistrar};
use application::usecases::translation::{
    BeginTranslationRequest, BeginTranslationUseCase, InspectTranslationSourceUseCase,
    ReconcileTranslationRunsUseCase,
};
use auralis_translation::{RunId, TranslationId};
use auralis_translation_sqlite::{SqliteConfig, TranslateDb};
use domain::media::ArtifactId;
use domain::project::Project;
use ports::repository::ProjectRepository;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationIntent, TranslationLinkStore};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);

#[tokio::test]
async fn begin_keeps_link_after_translate_database_failure_and_retries()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let auralis_path = directory.path().join("auralis.sqlite");
    let translate_directory = directory.path().join("translate-state");
    let translate_path = translate_directory.join("translate.sqlite");
    let pool = connect_sqlite(&auralis_path).await?;
    let projects = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifacts = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let links = Arc::new(SqliteTranslationLinkStore::new(pool.clone()));
    let file_store = Arc::new(LocalArtifactStore::new(directory.path().join("artifacts")));
    let project = projects.create(Project::new("Translation".into())?).await?;
    let original = b"1\n00:00:01,000 --> 00:00:02,000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_id = ArtifactId::new();
    let created_at = domain::chrono::Utc::now();
    let key = format!("projects/{}/original.srt", project.id());
    let source_path = file_store.resolve_storage_key(&key)?;
    std::fs::create_dir_all(source_path.parent().ok_or("missing source parent")?)?;
    std::fs::write(&source_path, original)?;
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'OriginalSubtitle', ?, ?, 'ready', ?, ?)")
        .bind(source_id.to_string())
        .bind(project.id().to_string())
        .bind(&key)
        .bind(i64::try_from(original.len())?)
        .bind(created_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool).await?;
    let inspector = Arc::new(InspectTranslationSourceUseCase::new(
        projects,
        artifacts,
        file_store,
        Arc::new(FormatInspector),
    ));
    let registrar = Arc::new(TranslateRunRegistrar::new(translate_path.clone()));
    let begin = BeginTranslationUseCase::new(inspector.clone(), links.clone(), registrar.clone());
    let reconcile = ReconcileTranslationRunsUseCase::new(inspector, links.clone(), registrar);
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let request = || BeginTranslationRequest {
        project_id: project.id().clone(),
        source_artifact_id: source_id.clone(),
        format: TranslationSourceFormat::Srt,
        source_language: "zh".into(),
        translation_id,
        run_id,
        profile_manifest_json: PROFILE.into(),
        created_at,
    };

    let first_error = match begin.execute(request()).await {
        Ok(_) => return Err("first start unexpectedly succeeded".into()),
        Err(error) => error,
    };
    assert!(
        first_error.to_string().contains("open_translate_database"),
        "{first_error}"
    );
    assert!(!translate_path.exists());
    let persisted = links.get(translation_id).await?.ok_or("missing link")?;
    assert_eq!(persisted.active_run_id, Some(run_id));
    assert!(links.run_intent(run_id).await?.is_some());
    std::fs::create_dir_all(&translate_directory)?;
    let recovery = reconcile.execute().await?;
    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].translation_id, translation_id);
    assert!(recovery[0].outcome.is_ok());
    let completed = begin.execute(request()).await?;
    assert_eq!(completed.translation_id, translation_id);
    assert_eq!(completed.active_run_id, Some(run_id));
    assert_eq!(std::fs::read(&source_path)?, original);

    let translated = TranslateDb::open(&translate_path, SqliteConfig::default())?;
    let translation =
        translated.translation(TranslationId::new(translation_id).ok_or("nil ID")?)?;
    assert_eq!(
        translation.project_id.as_deref(),
        Some(project.id().to_string().as_str())
    );
    assert_eq!(
        translated
            .run(RunId::new(run_id).ok_or("nil ID")?)?
            .blocks
            .len(),
        1
    );

    let legacy_id = Uuid::new_v4();
    links
        .begin(TranslationIntent {
            translation_id: legacy_id,
            project_id: project.id().clone(),
            source_artifact_id: source_id,
            source_sha256: completed.source_sha256,
            source_format: "srt".into(),
            source_language: "zh".into(),
            target_language: "ru".into(),
            run_id: Uuid::new_v4(),
            created_at,
        })
        .await?;
    let outcomes = reconcile.execute().await?;
    assert_eq!(outcomes.len(), 2);
    assert!(
        outcomes
            .iter()
            .any(|item| item.translation_id == translation_id && item.outcome.is_ok())
    );
    assert!(
        outcomes
            .iter()
            .any(|item| item.translation_id == legacy_id && item.outcome.is_err())
    );
    Ok(())
}
