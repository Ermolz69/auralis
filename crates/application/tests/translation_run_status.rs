#[path = "support/gated_translation_runtime.rs"]
#[expect(dead_code)]
mod gated_translation_runtime;
#[path = "support/translation_admission_fixture.rs"]
#[expect(dead_code)]
mod translation_admission_fixture;

use application::{
    error::ApplicationError,
    usecases::translation::{GetTranslationRunStatusUseCase, TranslationRunRequest},
};
use domain::{
    job::{Job, JobKind},
    project::ProjectId,
};
use gated_translation_runtime::GatedTranslationRuntime;
use ports::{
    translation_host_job::TranslationHostJobStore, translation_run_control::TranslationRunPhase,
};
use std::error::Error;
use translation_admission_fixture::TranslationAdmissionFixture;

#[tokio::test]
async fn status_retains_the_host_identity_until_preflight_job_terminalization()
-> Result<(), Box<dyn Error>> {
    let fixture = TranslationAdmissionFixture::new(GatedTranslationRuntime::new(
        "http://127.0.0.1:9/".into(),
    ))
    .await?;
    fixture.pause.pause(fixture.request.clone()).await?;
    let status = GetTranslationRunStatusUseCase::new(fixture.pause.clone(), fixture.jobs.clone());
    let idle = status.execute(fixture.request.clone()).await?;
    assert_eq!(idle.snapshot.phase, TranslationRunPhase::Paused);
    assert!(idle.active_host_job_id.is_none());
    let mut job = fixture
        .jobs
        .create_pending(
            Job::new(
                fixture.request.project_id.clone(),
                "Status preparation".into(),
                JobKind::Translation,
            ),
            fixture.request.translation_id,
            fixture.run_id,
        )
        .await?;
    let pending = status.execute(fixture.request.clone()).await?;
    assert_eq!(pending.snapshot, idle.snapshot);
    assert_eq!(pending.active_host_job_id.as_ref(), Some(job.id()));
    let revision = job.revision();
    job.start()?;
    fixture.jobs.start(&job, revision).await?;
    let running_host = status.execute(fixture.request.clone()).await?;
    assert_eq!(running_host.snapshot.phase, TranslationRunPhase::Paused);
    assert_eq!(running_host.active_host_job_id.as_ref(), Some(job.id()));
    let revision = job.revision();
    job.cancel()?;
    fixture.jobs.finish(&job, revision).await?;
    let released = status.execute(fixture.request.clone()).await?;
    assert_eq!(released.snapshot, idle.snapshot);
    assert!(released.active_host_job_id.is_none());
    let translate = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&fixture.translate_path)
            .read_only(true),
    )
    .await?;
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_attempts")
        .fetch_one(&translate)
        .await?;
    assert_eq!(attempts, 0);
    translate.close().await;
    Ok(())
}

#[tokio::test]
async fn status_rejects_another_project_and_propagates_host_storage_failure()
-> Result<(), Box<dyn Error>> {
    let fixture = TranslationAdmissionFixture::new(GatedTranslationRuntime::new(
        "http://127.0.0.1:9/".into(),
    ))
    .await?;
    let wrong = TranslationRunRequest {
        project_id: ProjectId::new(),
        translation_id: fixture.request.translation_id,
    };
    let status = GetTranslationRunStatusUseCase::new(fixture.pause.clone(), fixture.jobs.clone());
    assert!(matches!(
        status.execute(wrong).await,
        Err(ApplicationError::InvalidOperation { .. })
    ));
    sqlx::query("DROP TABLE translation_host_jobs")
        .execute(&fixture.pool)
        .await?;
    assert!(status.execute(fixture.request.clone()).await.is_err());
    Ok(())
}
