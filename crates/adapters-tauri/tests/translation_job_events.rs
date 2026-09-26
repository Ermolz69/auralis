#![allow(clippy::expect_used)]

#[path = "support/translation_run.rs"]
mod translation_run;

use adapters_storage::sqlite::{
    SqliteJobRepository, SqliteTranslationHostJobStore, connect_sqlite,
};
use adapters_tauri::{EventfulTranslationHostJobs, JobEventBridgeConfig, PreparedJobEventBridge};
use domain::job::{Job, JobError, JobKind};
use ports::job_scheduler::{JobLifecycleEvent, JobLifecycleEventKind};
use ports::repository::JobRepository;
use ports::translation_host_job::TranslationHostJobStore;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn notifications_observe_committed_revisions_and_skip_failed_writes()
-> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
    let (project, translation_id, run_id) = translation_run::seed_run(&pool).await?;
    let events = Arc::new(Mutex::new(Vec::<JobLifecycleEvent>::new()));
    let observed = events.clone();
    let reader_pool = pool.clone();
    let store = EventfulTranslationHostJobs::new(
        Arc::new(SqliteTranslationHostJobStore::new(pool.clone())),
        Arc::new(move |event| {
            let reader_pool = reader_pool.clone();
            let expected = event.job.clone();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().expect("event read runtime");
                runtime.block_on(async {
                    let committed = SqliteJobRepository::new(reader_pool)
                        .get(&expected.id)
                        .await
                        .expect("event read")
                        .expect("committed job");
                    assert_eq!(committed.revision(), expected.revision);
                    assert_eq!(committed.status(), &expected.status);
                    assert_eq!(committed.progress(), &expected.progress);
                    assert_eq!(Some(committed.project_id()), expected.project_id.as_ref());
                    assert_eq!(committed.kind(), &JobKind::Translation);
                    assert!(expected.stage.is_none());
                });
            })
            .join()
            .expect("event must already be visible to another reader");
            observed.lock().expect("events").push(event);
        }),
    );
    let mut job = store
        .create_pending(
            Job::new(
                project.id().clone(),
                "Translate".into(),
                JobKind::Translation,
            ),
            translation_id,
            run_id,
        )
        .await?;
    assert!(
        store
            .create_pending(
                Job::new(
                    project.id().clone(),
                    "Duplicate".into(),
                    JobKind::Translation
                ),
                translation_id,
                run_id,
            )
            .await
            .is_err()
    );
    assert_eq!(events.lock().map_err(|_| "event mutex poisoned")?.len(), 1);
    assert_eq!(store.list_active().await?.len(), 1);
    assert_eq!(store.active_for_run(run_id).await?, Some(job.clone()));
    let revision = job.revision();
    job.start()?;
    store.start(&job, revision).await?;
    assert!(store.start(&job, revision).await.is_err());
    let revision = job.revision();
    job.report_translation_progress(1, 2)?;
    store.record_progress(&job, revision).await?;
    assert!(store.record_progress(&job, revision).await.is_err());
    assert_eq!(events.lock().map_err(|_| "event mutex poisoned")?.len(), 3);
    let revision = job.revision();
    job.mark_completed()?;
    store.finish(&job, revision).await?;
    assert!(store.finish(&job, revision).await.is_err());
    assert!(store.active_for_run(run_id).await?.is_none());
    let events = events.lock().map_err(|_| "event mutex poisoned")?;
    assert_eq!(
        events.iter().map(|event| &event.kind).collect::<Vec<_>>(),
        vec![
            &JobLifecycleEventKind::Created,
            &JobLifecycleEventKind::Started,
            &JobLifecycleEventKind::Progressed,
            &JobLifecycleEventKind::Completed,
        ]
    );
    assert_eq!(events[2].job.progress.processed_items, Some(1));
    assert_eq!(events[2].job.progress.total_items, Some(2));
    assert!(events[2].job.progress.percent < 100);
    assert_eq!(events[3].job.progress.percent, 100);
    Ok(())
}

#[tokio::test]
async fn terminal_outcomes_are_notified_and_disconnected_bridge_does_not_fail_storage()
-> Result<(), Box<dyn Error>> {
    for failed in [false, true] {
        let directory = tempfile::tempdir()?;
        let pool = connect_sqlite(directory.path().join("auralis.sqlite")).await?;
        let (project, translation_id, run_id) = translation_run::seed_run(&pool).await?;
        let events = Arc::new(Mutex::new(Vec::new()));
        let observed = events.clone();
        let bridge = PreparedJobEventBridge::new(JobEventBridgeConfig::default());
        let disconnected = bridge.emitter();
        drop(bridge);
        let store = EventfulTranslationHostJobs::new(
            Arc::new(SqliteTranslationHostJobStore::new(pool.clone())),
            Arc::new(move |event| {
                disconnected(event.clone());
                observed.lock().expect("events").push(event);
            }),
        );
        let mut job = store
            .create_pending(
                Job::new(project.id().clone(), "Stop".into(), JobKind::Translation),
                translation_id,
                run_id,
            )
            .await?;
        assert!(store.finish(&job, job.revision()).await.is_err());
        assert_eq!(events.lock().map_err(|_| "event mutex poisoned")?.len(), 1);
        let revision = job.revision();
        if failed {
            job.mark_failed(JobError::new("provider", "Provider failed", true))?;
        } else {
            job.cancel()?;
        }
        store.finish(&job, revision).await?;
        assert!(store.list_active().await?.is_empty());
        assert_eq!(
            SqliteJobRepository::new(pool).get(job.id()).await?,
            Some(job.clone())
        );
        let events = events.lock().map_err(|_| "event mutex poisoned")?;
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[1].kind,
            if failed {
                JobLifecycleEventKind::Failed
            } else {
                JobLifecycleEventKind::Cancelled
            }
        );
        assert_eq!(
            events[1].job.error.as_deref(),
            if failed {
                Some("Provider failed")
            } else {
                None
            }
        );
    }
    Ok(())
}
