#[path = "support/completing_translation_runtime.rs"]
mod completing_translation_runtime;
#[path = "support/completion_translation_control.rs"]
mod completion_translation_control;
#[path = "support/translation_admission_fixture.rs"]
#[expect(dead_code)]
mod translation_admission_fixture;

use application::error::ApplicationError;
use completing_translation_runtime::CompletingTranslationRuntime;
use completion_translation_control::CompletionTranslationControl;
use ports::{error::PortError, translation_run_control::TranslationRunPhase};
use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use translation_admission_fixture::{SOURCE, TranslationAdmissionFixture};

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn completion_cleanup_failure_survives_new_pause_or_control_read_error()
-> Result<(), Box<dyn Error>> {
    for fail_control in [false, true] {
        let runtime = CompletingTranslationRuntime::new(true);
        let completed = runtime.completed.clone();
        let checked = Arc::new(AtomicBool::new(false));
        let observed = checked.clone();
        let fixture = TranslationAdmissionFixture::new_with_control(runtime, move |control| {
            Arc::new(CompletionTranslationControl::new(
                control,
                completed,
                checked,
                fail_control,
            ))
        })
        .await?;
        let outcome =
            tokio::time::timeout(TIMEOUT, fixture.managed.admit(fixture.request.clone())).await?;
        assert!(
            observed.load(Ordering::SeqCst),
            "The completion guard was not checked"
        );
        match outcome {
            Err(ApplicationError::Port(PortError::CleanupFailed { resource, message })) => {
                assert_eq!(resource, "completion test runtime");
                assert_eq!(message, "cleanup failed at acquisition completion");
            }
            _ => panic!("Final control changes hid the runtime cleanup failure"),
        }
        assert_no_admission(&fixture).await?;
    }
    Ok(())
}

#[tokio::test]
async fn completion_runtime_failure_obeys_the_final_guard() -> Result<(), Box<dyn Error>> {
    for fail_control in [false, true] {
        let runtime = CompletingTranslationRuntime::new(false);
        let completed = runtime.completed.clone();
        let checked = Arc::new(AtomicBool::new(false));
        let observed = checked.clone();
        let fixture = TranslationAdmissionFixture::new_with_control(runtime, move |control| {
            Arc::new(CompletionTranslationControl::new(
                control,
                completed,
                checked,
                fail_control,
            ))
        })
        .await?;
        let outcome =
            tokio::time::timeout(TIMEOUT, fixture.managed.admit(fixture.request.clone())).await?;
        assert!(observed.load(Ordering::SeqCst));
        if fail_control {
            assert!(matches!(
                outcome,
                Err(ApplicationError::Port(PortError::Storage {
                    operation: "completion control read",
                    ..
                }))
            ));
        } else {
            assert!(matches!(
                outcome,
                Err(ApplicationError::Port(PortError::Cancelled))
            ));
        }
        assert_no_admission(&fixture).await?;
    }
    Ok(())
}

async fn assert_no_admission(fixture: &TranslationAdmissionFixture) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        fixture.pause.status(fixture.request.clone()).await?.phase,
        TranslationRunPhase::Paused
    );
    let host: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM jobs), (SELECT COUNT(*) FROM translation_host_jobs), (SELECT COUNT(*) FROM translation_publications)")
        .fetch_one(&fixture.pool).await?;
    assert_eq!(host, (0, 0, 0));
    let translate = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&fixture.translate_path)
            .read_only(true),
    )
    .await?;
    let saved: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM run_attempts), (SELECT COUNT(*) FROM block_checkpoints), (SELECT COUNT(*) FROM results)")
        .fetch_one(&translate).await?;
    assert_eq!(saved, (0, 0, 0));
    translate.close().await;
    assert_eq!(std::fs::read(&fixture.source_path)?, SOURCE);
    Ok(())
}
