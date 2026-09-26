#[path = "support/gated_translation_runtime.rs"]
mod gated_translation_runtime;
#[path = "support/translation_admission_fixture.rs"]
mod translation_admission_fixture;

use application::error::ApplicationError;
use gated_translation_runtime::GatedTranslationRuntime;
use ports::{
    error::PortError, translation_host_job::TranslationHostJobStore,
    translation_links::TranslationLinkStore, translation_run_control::TranslationRunPhase,
};
use std::{error::Error, net::TcpListener, sync::atomic::Ordering, time::Duration};
use translation_admission_fixture::{SOURCE, TranslationAdmissionFixture};

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn pause_does_not_hide_failed_preparation_cleanup() -> Result<(), Box<dyn Error>> {
    let runtime = GatedTranslationRuntime::cleanup_failure("http://127.0.0.1:9/".into());
    let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
    let managed = fixture.managed.clone();
    let request = fixture.request.clone();
    let admission = tokio::spawn(async move { managed.admit(request).await });
    tokio::time::timeout(TIMEOUT, runtime.entered.acquire())
        .await??
        .forget();
    fixture.pause.pause(fixture.request.clone()).await?;
    assert!(matches!(
        tokio::time::timeout(TIMEOUT, admission).await??,
        Err(ApplicationError::Port(PortError::CleanupFailed { .. }))
    ));
    assert!(fixture.jobs.active_for_run(fixture.run_id).await?.is_none());
    assert_eq!(
        fixture.pause.status(fixture.request.clone()).await?.phase,
        TranslationRunPhase::Paused
    );
    Ok(())
}

#[tokio::test]
async fn pause_stops_preparation_without_releasing_the_runtime_barrier()
-> Result<(), Box<dyn Error>> {
    for initially_paused in [false, true] {
        let runtime = GatedTranslationRuntime::cancellable("http://127.0.0.1:9/".into());
        let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
        if initially_paused {
            fixture.pause.pause(fixture.request.clone()).await?;
        }
        let managed = fixture.managed.clone();
        let request = fixture.request.clone();
        let admission = tokio::spawn(async move { managed.admit(request).await });
        tokio::time::timeout(TIMEOUT, runtime.entered.acquire())
            .await??
            .forget();
        fixture.pause.pause(fixture.request.clone()).await?;
        assert!(matches!(
            tokio::time::timeout(TIMEOUT, admission).await??,
            Err(ApplicationError::Port(PortError::Cancelled))
        ));
        assert_eq!(runtime.returned.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.dropped.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.release.available_permits(), 0);
        assert!(fixture.jobs.active_for_run(fixture.run_id).await?.is_none());
        assert_eq!(
            fixture.pause.status(fixture.request.clone()).await?.phase,
            TranslationRunPhase::Paused
        );
        assert_eq!(std::fs::read(&fixture.source_path)?, SOURCE);
    }
    Ok(())
}

#[test]
fn admission_policy_rejects_unbounded_polling() {
    use application::usecases::translation::TranslationAdmissionPolicy;
    assert!(TranslationAdmissionPolicy::new(Duration::ZERO).is_none());
    assert!(TranslationAdmissionPolicy::new(Duration::from_millis(9)).is_none());
    assert!(TranslationAdmissionPolicy::new(Duration::from_millis(10)).is_some());
    assert!(TranslationAdmissionPolicy::new(Duration::from_secs(1)).is_some());
    assert!(TranslationAdmissionPolicy::new(Duration::from_millis(1001)).is_none());
}

#[tokio::test]
async fn pause_during_model_admission_prevents_creating_a_host_job() -> Result<(), Box<dyn Error>> {
    for initially_paused in [false, true] {
        let runtime = GatedTranslationRuntime::new("http://127.0.0.1:9/".into());
        let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
        if initially_paused {
            fixture.pause.pause(fixture.request.clone()).await?;
        }
        let managed = fixture.managed.clone();
        let request = fixture.request.clone();
        let admission = tokio::spawn(async move { managed.admit(request).await });
        tokio::time::timeout(TIMEOUT, runtime.entered.acquire())
            .await??
            .forget();
        fixture.pause.pause(fixture.request.clone()).await?;
        runtime.release.add_permits(1);
        let result = tokio::time::timeout(TIMEOUT, admission).await??;
        assert!(
            matches!(result, Err(ApplicationError::Port(PortError::Cancelled))),
            "A newer pause was treated as permission to resume"
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM translation_host_jobs")
            .fetch_one(&fixture.pool)
            .await?;
        assert_eq!(count, 0);
        assert_eq!(runtime.returned.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.dropped.load(Ordering::SeqCst), 1);
        assert_eq!(
            fixture.pause.status(fixture.request.clone()).await?.phase,
            TranslationRunPhase::Paused
        );
        assert_eq!(std::fs::read(&fixture.source_path)?, SOURCE);
    }
    Ok(())
}

#[tokio::test]
async fn newer_pause_wins_over_a_runtime_admission_failure() -> Result<(), Box<dyn Error>> {
    let runtime = GatedTranslationRuntime::failing("http://127.0.0.1:9/".into());
    let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
    let managed = fixture.managed.clone();
    let request = fixture.request.clone();
    let admission = tokio::spawn(async move { managed.admit(request).await });
    tokio::time::timeout(TIMEOUT, runtime.entered.acquire())
        .await??
        .forget();
    fixture.pause.pause(fixture.request.clone()).await?;
    runtime.release.add_permits(1);
    assert!(matches!(
        tokio::time::timeout(TIMEOUT, admission).await??,
        Err(ApplicationError::Port(PortError::Cancelled))
    ));
    assert_eq!(
        fixture.pause.status(fixture.request.clone()).await?.phase,
        TranslationRunPhase::Paused
    );
    assert!(fixture.jobs.active_for_run(fixture.run_id).await?.is_none());
    assert_eq!(runtime.dropped.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn runtime_failure_without_a_new_pause_preserves_its_error() -> Result<(), Box<dyn Error>> {
    let runtime = GatedTranslationRuntime::failing("http://127.0.0.1:9/".into());
    let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
    runtime.release.add_permits(1);
    assert!(matches!(
        fixture.managed.admit(fixture.request.clone()).await,
        Err(ApplicationError::Port(PortError::ExternalToolFailed { .. }))
    ));
    assert_eq!(
        fixture.pause.status(fixture.request.clone()).await?.phase,
        TranslationRunPhase::Requested
    );
    assert!(fixture.jobs.active_for_run(fixture.run_id).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn pause_after_admission_cancels_the_host_job_before_inference() -> Result<(), Box<dyn Error>>
{
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let runtime = GatedTranslationRuntime::new(format!("http://{}/", listener.local_addr()?));
    let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
    runtime.release.add_permits(1);
    let attempt = fixture.managed.admit(fixture.request.clone()).await?;
    let job_id = attempt.job().id().clone();
    assert_eq!(attempt.run_id(), fixture.run_id);
    fixture.pause.pause(fixture.request.clone()).await?;
    assert!(matches!(
        tokio::time::timeout(TIMEOUT, attempt.execute()).await?,
        Err(ApplicationError::Port(PortError::Cancelled))
    ));
    let transport_error = listener
        .accept()
        .err()
        .ok_or("No HTTP request should start")?;
    assert_eq!(transport_error.kind(), std::io::ErrorKind::WouldBlock);
    assert_eq!(runtime.dropped.load(Ordering::SeqCst), 1);
    assert!(fixture.jobs.active_for_run(fixture.run_id).await?.is_none());
    let status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
        .bind(job_id.to_string())
        .fetch_one(&fixture.pool)
        .await?;
    assert_eq!(status, "cancelled");
    let link = fixture
        .links
        .get(fixture.request.translation_id)
        .await?
        .ok_or("missing link")?;
    assert_eq!(link.active_run_id, Some(fixture.run_id));
    assert!(link.selected_result_id.is_none());
    let translate = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&fixture.translate_path)
            .read_only(true),
    )
    .await?;
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_attempts")
        .fetch_one(&translate)
        .await?;
    let results: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM results")
        .fetch_one(&translate)
        .await?;
    assert_eq!((attempts, results), (0, 0));
    translate.close().await;
    assert_eq!(std::fs::read(&fixture.source_path)?, SOURCE);
    Ok(())
}

#[tokio::test]
async fn a_fresh_explicit_resume_can_be_admitted_after_pause() -> Result<(), Box<dyn Error>> {
    let runtime = GatedTranslationRuntime::new("http://127.0.0.1:9/".into());
    let fixture = TranslationAdmissionFixture::new(runtime.clone()).await?;
    fixture.pause.pause(fixture.request.clone()).await?;
    runtime.release.add_permits(1);
    let attempt = fixture.managed.admit(fixture.request.clone()).await?;
    assert_eq!(attempt.run_id(), fixture.run_id);
    assert_eq!(
        fixture
            .jobs
            .active_for_run(fixture.run_id)
            .await?
            .ok_or("missing job")?
            .id(),
        attempt.job().id()
    );
    fixture.pause.pause(fixture.request.clone()).await?;
    assert!(matches!(
        attempt.execute().await,
        Err(ApplicationError::Port(PortError::Cancelled))
    ));
    assert_eq!(runtime.dropped.load(Ordering::SeqCst), 1);
    Ok(())
}
