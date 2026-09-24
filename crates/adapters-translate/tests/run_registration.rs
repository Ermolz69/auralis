use adapters_translate::TranslateRunRegistrar;
use auralis_translation::{BlockPolicy, RunId, SourceHash, TranslationId};
use auralis_translation_formats::srt::SrtRunPlan;
use auralis_translation_formats::vtt::VttRunPlan;
use auralis_translation_sqlite::{SqliteConfig, TranslateDb};
use chrono::Utc;
use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::translation_links::{TranslationLink, TranslationRunIntent};
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::error::Error;
use uuid::Uuid;

const PROFILE: &str = include_str!(
    "../../../modules/auralis-translate/models/manifests/hy_mt2_1_8b_q4_k_m.experimental.json"
);

fn records(format: &str, source: &[u8]) -> (TranslationLink, TranslationRunIntent) {
    let translation_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let now = Utc::now();
    let policy = BlockPolicy::default();
    let policy_fingerprint = match format {
        "srt" => SrtRunPlan::policy_fingerprint(policy),
        "vtt" => VttRunPlan::policy_fingerprint(policy),
        _ => unreachable!(),
    };
    (
        TranslationLink {
            translation_id,
            project_id: ProjectId::new(),
            source_artifact_id: ArtifactId::new(),
            source_sha256: SourceHash::digest(source).to_string(),
            source_format: format.into(),
            source_language: "zh".into(),
            target_language: "ru".into(),
            active_run_id: Some(run_id),
            selected_result_id: None,
            output_artifact_id: None,
            revision: 1,
            created_at: now,
            updated_at: now,
        },
        TranslationRunIntent {
            run_id,
            translation_id,
            profile_manifest_json: PROFILE.into(),
            profile_sha256: SourceHash::digest(PROFILE.as_bytes()).to_string(),
            max_target_segments: policy.max_target_segments() as u32,
            context_before_segments: policy.context_before_segments() as u32,
            context_after_segments: policy.context_after_segments() as u32,
            parser_version: 1,
            policy_fingerprint: policy_fingerprint.to_string(),
            created_at: now,
        },
    )
}

#[tokio::test]
async fn registration_is_idempotent_and_preserves_original() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"1\r\n00:00:01,000 --> 00:00:02,000\r\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\r\n";
    let source_path = directory.path().join("source.srt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("srt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    registrar.ensure_run(&source_path, &link, &intent).await?;
    registrar.ensure_run(&source_path, &link, &intent).await?;

    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let translation_id = TranslationId::new(link.translation_id).ok_or("nil translation ID")?;
    let run_id = RunId::new(intent.run_id).ok_or("nil run ID")?;
    let translation = database.translation(translation_id)?;
    assert_eq!(
        translation.project_id.as_deref(),
        Some(link.project_id.to_string().as_str())
    );
    assert_eq!(
        translation.source_artifact_id.as_deref(),
        Some(link.source_artifact_id.to_string().as_str())
    );
    assert_eq!(database.segments(translation_id)?.len(), 1);
    assert_eq!(database.run(run_id)?.blocks.len(), 1);
    assert_eq!(std::fs::read(&source_path)?, source);
    Ok(())
}

#[tokio::test]
async fn registration_rejects_changed_original_before_database_creation()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"WEBVTT\n\n00:01.000 --> 00:02.000\nhello\n";
    let source_path = directory.path().join("source.vtt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(
        &source_path,
        b"WEBVTT\n\n00:01.000 --> 00:02.000\nchanged\n",
    )?;
    let (link, intent) = records("vtt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    assert!(
        registrar
            .ensure_run(&source_path, &link, &intent)
            .await
            .is_err()
    );
    assert!(!database_path.exists());
    Ok(())
}

#[tokio::test]
async fn webvtt_registration_keeps_optional_cue_identity_and_rejects_changed_profile()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let source = b"WEBVTT\n\nNOTE provenance\nsynthetic\n\n00:01.000 --> 00:02.000\n\xe4\xbd\xa0\xe5\xa5\xbd\xe3\x80\x82\n";
    let source_path = directory.path().join("source.vtt");
    let database_path = directory.path().join("translate.sqlite");
    std::fs::write(&source_path, source)?;
    let (link, intent) = records("vtt", source);
    let registrar = TranslateRunRegistrar::new(database_path.clone());
    registrar.ensure_run(&source_path, &link, &intent).await?;
    let database = TranslateDb::open(&database_path, SqliteConfig::default())?;
    let translation_id = TranslationId::new(link.translation_id).ok_or("nil ID")?;
    let segments = database.segments(translation_id)?;
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].cue_label, None);
    drop(database);

    let mut changed = intent.clone();
    changed.profile_manifest_json.push(' ');
    assert!(
        registrar
            .ensure_run(&source_path, &link, &changed)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&source_path)?, source);
    Ok(())
}
