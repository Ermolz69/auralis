#![allow(clippy::unwrap_used)]

use super::{orphan_writes::commit_failed_orphan_job, pair_writes::commit_failed_interrupted_pair};
use crate::sqlite::{SqliteJobRepository, SqliteProjectRepository, connect_sqlite};
use domain::{
    job::{Job, JobError, JobKind},
    project::Project,
};
use ports::{
    error::PortError,
    recovery::{FailInterruptedPairCommand, FailOrphanJobCommand, RecoveryApplyResult},
    repository::{JobRepository, ProjectRepository},
};

#[tokio::test]
async fn recovery_persists_complete_terminal_snapshot_and_rejects_stale_writes() {
    for paired in [false, true] {
        for cancelling in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("recovery.sqlite");
            let pool = connect_sqlite(&path).await.unwrap();
            let project = Project::new("Recovery".into()).unwrap();
            let mut job = Job::new(project.id().clone(), "Job".into(), JobKind::Dubbing);
            job.start().unwrap();
            if cancelling {
                job.request_cancellation().unwrap();
            }
            let mut snapshot = project.to_snapshot();
            snapshot.source = Some(domain::media::MediaSource::ExternalLocalFile {
                path: "fixture".into(),
            });
            snapshot.status = if paired {
                domain::project::ProjectStatus::Processing
            } else {
                domain::project::ProjectStatus::ReadyForProcessing
            };
            snapshot.active_job_id = paired.then(|| job.id().clone());
            let mut project = Project::from_snapshot(snapshot).unwrap();
            SqliteProjectRepository::new(pool.clone())
                .create(project.clone())
                .await
                .unwrap();
            SqliteJobRepository::new(pool.clone())
                .create(job.clone())
                .await
                .unwrap();
            let original = job.clone();
            job.mark_failed(JobError::new("APP_RESTART", "Interrupted", false))
                .unwrap();
            if paired {
                project
                    .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
                    .unwrap();
            }
            for expected in [
                RecoveryApplyResult::Applied,
                RecoveryApplyResult::AlreadyApplied,
            ] {
                let result = if paired {
                    commit_failed_interrupted_pair(
                        &pool,
                        FailInterruptedPairCommand {
                            expected_job_revision: original.revision(),
                            expected_job_status: original.status().clone(),
                            project: project.clone(),
                            job: job.clone(),
                            expected_project_status: domain::project::ProjectStatus::Processing,
                            expected_active_job_id: job.id().clone(),
                            expected_last_terminal_job_id: None,
                        },
                    )
                    .await
                } else {
                    commit_failed_orphan_job(
                        &pool,
                        FailOrphanJobCommand {
                            expected_job_revision: original.revision(),
                            expected_job_status: original.status().clone(),
                            job: job.clone(),
                        },
                    )
                    .await
                };
                assert_eq!(result.unwrap(), expected);
            }
            pool.close().await;
            let reopened = connect_sqlite(&path).await.unwrap();
            let repo = SqliteJobRepository::new(reopened.clone());
            let stored = repo.get(job.id()).await.unwrap().unwrap();
            assert_eq!(stored, job);
            assert_eq!(stored.revision(), original.revision() + 1);
            assert_eq!(stored.finished_at(), Some(stored.updated_at()));
            assert!(matches!(
                repo.save(&original, original.revision()).await,
                Err(PortError::Conflict { .. })
            ));
            reopened.close().await;
        }
    }
}

#[tokio::test]
async fn recovery_rejects_same_status_with_a_newer_revision() {
    let root = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(root.path().join("recovery.sqlite"))
        .await
        .unwrap();
    let project = Project::new("Recovery".into()).unwrap();
    SqliteProjectRepository::new(pool.clone())
        .create(project.clone())
        .await
        .unwrap();
    let mut job = Job::new(project.id().clone(), "Job".into(), JobKind::Dubbing);
    job.start().unwrap();
    let repo = SqliteJobRepository::new(pool.clone());
    repo.create(job.clone()).await.unwrap();
    let expected_job_revision = job.revision();
    sqlx::query("UPDATE jobs SET revision = revision + 1")
        .execute(&pool)
        .await
        .unwrap();
    job.mark_failed(JobError::new("APP_RESTART", "Interrupted", false))
        .unwrap();
    assert!(matches!(
        commit_failed_orphan_job(
            &pool,
            FailOrphanJobCommand {
                job: job.clone(),
                expected_job_revision,
                expected_job_status: domain::job::JobStatus::Running,
            }
        )
        .await,
        Err(PortError::Conflict { .. })
    ));
    assert_eq!(
        repo.get(job.id()).await.unwrap().unwrap().status(),
        &domain::job::JobStatus::Running
    );
    pool.close().await;
}
