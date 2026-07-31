#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
// Module is defined externally as tests.rs
use crate::sqlite::recovery::pair_writes::commit_failed_interrupted_pair;
use crate::sqlite::recovery::snapshot::load_snapshot;
use domain::job::{Job, JobKind, JobStatus};
use domain::project::{Project, ProjectStatus};
use ports::error::PortError;
use ports::recovery::{FailInterruptedPairCommand, RecoveryApplyResult};
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

fn enum_text<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap()
        .trim_matches('"')
        .to_string()
}

async fn insert_project(
    pool: &SqlitePool,
    project: &Project,
    status: ProjectStatus,
    active_job_id: Option<String>,
    last_terminal_job_id: Option<String>,
) {
    sqlx::query(
        "INSERT INTO projects (
            id, title, status, source_json, active_job_id, last_terminal_job_id, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(project.id().to_string())
    .bind("Title")
    .bind(enum_text(&status))
    .bind(serde_json::to_string(&domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    }).unwrap())
    .bind(active_job_id)
    .bind(last_terminal_job_id)
    .bind(project.created_at().to_rfc3339())
    .bind(project.updated_at().to_rfc3339())
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_job(pool: &SqlitePool, job: &Job, status: JobStatus) {
    sqlx::query(
        "INSERT INTO jobs (
            id, project_id, title, kind, status, progress_json, error_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(job.id().to_string())
    .bind(job.project_id().to_string())
    .bind(job.title())
    .bind(enum_text(job.kind()))
    .bind(enum_text(&status))
    .bind(serde_json::to_string(job.progress()).unwrap())
    .bind(job.error().map(|e| serde_json::to_string(e).unwrap()))
    .bind(job.created_at().to_rfc3339())
    .bind(job.updated_at().to_rfc3339())
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_fresh_migrated_db_snapshot_is_noop() {
    let pool = setup_db().await;

    let snapshot = load_snapshot(&pool).await.unwrap();

    assert!(snapshot.processing_projects.is_empty());
    assert!(snapshot.linked_jobs.is_empty());
    assert!(snapshot.active_jobs.is_empty());
}

#[tokio::test]
async fn test_already_applied_partial_pair() {
    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(job.id().to_string()),
        None,
    )
    .await;
    insert_job(&pool, &job, JobStatus::Running).await;

    // Now simulate another worker applied the change to JOB ONLY (partial already applied)
    sqlx::query("UPDATE jobs SET status = 'failed' WHERE id = ?")
        .bind(job.id().to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Prepare command to fail pair
    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    let _ = project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    let cmd = FailInterruptedPairCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
        expected_active_job_id: expected_active.clone(),
    };

    let res = commit_failed_interrupted_pair(&pool, cmd).await.unwrap();
    assert!(matches!(res, RecoveryApplyResult::Applied));

    // If we run it AGAIN, BOTH will be 0, and it should return AlreadyApplied.
    let cmd2 = FailInterruptedPairCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
        expected_active_job_id: expected_active,
    };
    let res2 = commit_failed_interrupted_pair(&pool, cmd2).await.unwrap();
    assert!(matches!(res2, RecoveryApplyResult::AlreadyApplied));
}

#[tokio::test]
async fn test_snapshot_apply_reload_interrupted_pair() {
    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(expected_active.to_string()),
        None,
    )
    .await;
    insert_job(&pool, &job, JobStatus::Running).await;

    let snapshot = load_snapshot(&pool).await.unwrap();
    assert_eq!(snapshot.processing_projects.len(), 1);
    assert_eq!(snapshot.linked_jobs.len(), 1);

    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    let cmd = FailInterruptedPairCommand {
        project,
        job,
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
        expected_active_job_id: expected_active,
    };

    let res = commit_failed_interrupted_pair(&pool, cmd).await.unwrap();
    assert!(matches!(res, RecoveryApplyResult::Applied));

    let snapshot = load_snapshot(&pool).await.unwrap();
    assert!(snapshot.processing_projects.is_empty());
    assert!(snapshot.linked_jobs.is_empty());
    assert!(snapshot.active_jobs.is_empty());
}

#[tokio::test]
async fn test_corrupted_processing_project_maps_to_invalid_stored_data() {
    let pool = setup_db().await;
    let project = Project::new("Proj1".into());

    insert_project(&pool, &project, ProjectStatus::Processing, None, None).await;
    sqlx::query("UPDATE projects SET created_at = 'not-a-date' WHERE id = ?")
        .bind(project.id().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let err = match load_snapshot(&pool).await {
        Ok(_) => panic!("expected corrupted row to fail"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        PortError::InvalidStoredData { field, .. } if field == "created_at"
    ));
}

#[tokio::test]
async fn test_second_pair_write_failure_rolls_back_first_write() {
    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(expected_active.to_string()),
        None,
    )
    .await;
    insert_job(&pool, &job, JobStatus::Running).await;

    sqlx::query(
        "CREATE TRIGGER fail_recovery_project_update
         BEFORE UPDATE ON projects
         BEGIN
             SELECT RAISE(ABORT, 'forced project update failure');
         END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    let cmd = FailInterruptedPairCommand {
        project,
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
        expected_active_job_id: expected_active,
    };

    let err = commit_failed_interrupted_pair(&pool, cmd)
        .await
        .unwrap_err();
    assert!(matches!(err, PortError::Storage { .. }));

    let job_status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
        .bind(job.id().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(job_status, enum_text(&JobStatus::Running));
}

#[tokio::test]
async fn test_already_applied_legacy_fallback() {
    use crate::sqlite::recovery::pair_writes::commit_legacy_pair_fallback;
    use ports::recovery::FailLegacyPairFallbackCommand;

    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = None;
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    let mut job = Job::new(project.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();

    insert_project(&pool, &project, ProjectStatus::Processing, None, None).await;
    insert_job(&pool, &job, JobStatus::Running).await;

    // Simulate Already Applied logic - update BOTH project and job exactly as the command would
    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    project.force_fail_legacy_recovery();

    sqlx::query("UPDATE projects SET status = ?, active_job_id = ? WHERE id = ?")
        .bind(enum_text(&ProjectStatus::Failed))
        .bind(None::<String>)
        .bind(project.id().to_string())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET status = 'failed' WHERE id = ?")
        .bind(job.id().to_string())
        .execute(&pool)
        .await
        .unwrap();

    let cmd = FailLegacyPairFallbackCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
    };

    let res = commit_legacy_pair_fallback(&pool, cmd).await.unwrap();
    assert!(matches!(res, RecoveryApplyResult::AlreadyApplied));
}

#[tokio::test]
async fn test_concurrent_zero_row_update() {
    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(job.id().to_string()),
        None,
    )
    .await;
    insert_job(&pool, &job, JobStatus::Running).await;

    // Simulate CONFLICT: Project changed to something completely different
    sqlx::query("UPDATE projects SET status = ?, active_job_id = NULL WHERE id = ?")
        .bind(enum_text(&ProjectStatus::Completed))
        .bind(project.id().to_string())
        .execute(&pool)
        .await
        .unwrap();

    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    let _ = project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    let cmd = FailInterruptedPairCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None,
        expected_active_job_id: expected_active.clone(),
    };

    let err = commit_failed_interrupted_pair(&pool, cmd)
        .await
        .unwrap_err();
    assert!(matches!(err, ports::error::PortError::Conflict { .. }));

    // Verify rollback (job should still be running in the DB)
    let job_status: String = sqlx::query_scalar("SELECT status FROM jobs WHERE id = ?")
        .bind(job.id().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(job_status, "running");
}

#[tokio::test]
async fn test_terminal_pair_reconciliation() {
    use crate::sqlite::recovery::pair_writes::commit_reconciled_terminal_pair;
    use ports::recovery::ReconcileTerminalPairCommand;

    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    job.mark_failed(domain::job::JobError::new("ERR", "Failed", false))
        .unwrap();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();
    project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(job.id().to_string()),
        None,
    )
    .await;
    insert_job(&pool, &job, JobStatus::Failed).await;

    let cmd = ReconcileTerminalPairCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_active_job_id: expected_active,
        expected_job_status: JobStatus::Failed,
        expected_last_terminal_job_id: None,
    };

    let res = commit_reconciled_terminal_pair(&pool, cmd).await.unwrap();
    assert!(matches!(res, RecoveryApplyResult::Applied));
}

#[tokio::test]
async fn test_missing_linked_job_adapter_write() {
    use crate::sqlite::recovery::pair_writes::commit_failed_project_with_missing_linked_job;
    use ports::recovery::FailProjectWithMissingLinkedJobCommand;

    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let missing_job_id = domain::job::JobId::new();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(missing_job_id.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(missing_job_id.to_string()),
        None,
    )
    .await;

    project.force_fail_legacy_recovery();

    let cmd = FailProjectWithMissingLinkedJobCommand {
        project: project.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_active_job_id: missing_job_id,
        expected_last_terminal_job_id: None,
    };

    let res = commit_failed_project_with_missing_linked_job(&pool, cmd)
        .await
        .unwrap();
    assert!(matches!(res, RecoveryApplyResult::Applied));
}

#[tokio::test]
async fn test_legacy_project_without_job() {
    use crate::sqlite::recovery::pair_writes::commit_failed_legacy_project_without_job;
    use ports::recovery::FailLegacyProjectWithoutJobCommand;

    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = None;
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(&pool, &project, ProjectStatus::Processing, None, None).await;

    project.force_fail_legacy_recovery();

    let cmd = FailLegacyProjectWithoutJobCommand {
        project: project.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_last_terminal_job_id: None,
    };

    let res = commit_failed_legacy_project_without_job(&pool, cmd)
        .await
        .unwrap();
    assert!(matches!(res, RecoveryApplyResult::Applied));
}

#[tokio::test]
async fn test_orphan_adapter_conflict() {
    use crate::sqlite::recovery::orphan_writes::commit_failed_orphan_job;
    use ports::recovery::FailOrphanJobCommand;

    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();

    insert_project(&pool, &tmp_p, ProjectStatus::Processing, None, None).await;
    insert_job(&pool, &job, JobStatus::Running).await;

    // Simulate concurrent update on the job
    sqlx::query("UPDATE jobs SET status = ? WHERE id = ?")
        .bind(enum_text(&JobStatus::Completed))
        .bind(job.id().to_string())
        .execute(&pool)
        .await
        .unwrap();

    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();

    let cmd = FailOrphanJobCommand {
        job: job.clone(),
        expected_job_status: JobStatus::Running,
    };

    let err = commit_failed_orphan_job(&pool, cmd).await.unwrap_err();
    assert!(matches!(err, ports::error::PortError::Conflict { .. }));
}

#[tokio::test]
async fn test_last_terminal_job_id_conflict() {
    let pool = setup_db().await;

    let tmp_p = Project::new("Proj1".into());
    let mut job = Job::new(tmp_p.id().clone(), "Title".into(), JobKind::Dubbing);
    let _ = job.start();
    let expected_active = job.id().clone();

    let mut snap = tmp_p.to_snapshot();
    snap.status = ProjectStatus::Processing;
    snap.active_job_id = Some(expected_active.clone());
    snap.source = Some(domain::media::MediaSource::ExternalLocalFile {
        path: "test".into(),
    });
    let mut project = Project::from_snapshot(snap).unwrap();

    insert_project(
        &pool,
        &project,
        ProjectStatus::Processing,
        Some(job.id().to_string()),
        Some(domain::job::JobId::new().to_string()),
    )
    .await;
    insert_job(&pool, &job, JobStatus::Running).await;

    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    let _ = project
        .apply_terminal_transition(job.id(), domain::job::TerminalOutcome::Failed)
        .unwrap();

    let cmd = FailInterruptedPairCommand {
        project: project.clone(),
        job: job.clone(),
        expected_project_status: ProjectStatus::Processing,
        expected_job_status: JobStatus::Running,
        expected_last_terminal_job_id: None, // We expected None, but DB has something else!
        expected_active_job_id: expected_active.clone(),
    };

    let err = commit_failed_interrupted_pair(&pool, cmd)
        .await
        .unwrap_err();
    assert!(matches!(err, ports::error::PortError::Conflict { .. }));
}
