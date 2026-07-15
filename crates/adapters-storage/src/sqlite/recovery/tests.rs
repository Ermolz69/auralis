#[cfg(test)]
// Module is defined externally as tests.rs
use crate::sqlite::recovery::pair_writes::commit_failed_interrupted_pair;
use domain::job::{Job, JobKind, JobStatus};
use domain::project::{Project, ProjectStatus};
use ports::recovery::{FailInterruptedPairCommand, RecoveryApplyResult};
use sqlx::SqlitePool;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE projects (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                active_job_id TEXT,
                last_terminal_job_id TEXT,
                language_code TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                schema_version INTEGER NOT NULL
            );",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE jobs (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                progress_json TEXT,
                error_json TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
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

    // Insert them as they were BEFORE crash
    sqlx::query("INSERT INTO projects (id, title, status, active_job_id, language_code, created_at, updated_at, schema_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(project.id().to_string())
            .bind("Title")
            .bind("Processing")
            .bind(job.id().to_string())
            .bind("en")
            .bind(project.created_at().to_rfc3339())
            .bind(project.updated_at().to_rfc3339())
            .bind(1)
            .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO jobs (id, project_id, kind, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(job.id().to_string())
            .bind(project.id().to_string())
            .bind("dubbing")
            .bind("running") // Status before crash
            .bind("{}")
            .bind(None::<String>)
            .bind(job.created_at().to_rfc3339())
            .bind(job.updated_at().to_rfc3339())
            .execute(&pool).await.unwrap();

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
        expected_active_job_id: expected_active,
    };
    let res2 = commit_failed_interrupted_pair(&pool, cmd2).await.unwrap();
    assert!(matches!(res2, RecoveryApplyResult::AlreadyApplied));
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

    // Insert as before crash
    sqlx::query("INSERT INTO projects (id, title, status, active_job_id, language_code, created_at, updated_at, schema_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(project.id().to_string())
            .bind("Title")
            .bind("Processing")
            .bind(None::<String>)
            .bind("en")
            .bind(project.created_at().to_rfc3339())
            .bind(project.updated_at().to_rfc3339())
            .bind(1)
            .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO jobs (id, project_id, kind, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(job.id().to_string())
            .bind(project.id().to_string())
            .bind("dubbing")
            .bind("running")
            .bind("{}")
            .bind(None::<String>)
            .bind(job.created_at().to_rfc3339())
            .bind(job.updated_at().to_rfc3339())
            .execute(&pool).await.unwrap();

    // Simulate Already Applied logic - update BOTH project and job exactly as the command would
    job.mark_failed(domain::job::JobError::new("ERR", "Interrupted", false))
        .unwrap();
    project.force_fail_legacy_recovery();

    sqlx::query("UPDATE projects SET status = ?, active_job_id = ? WHERE id = ?")
        .bind("Failed")
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

    // Insert
    sqlx::query("INSERT INTO projects (id, title, status, active_job_id, language_code, created_at, updated_at, schema_version) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(project.id().to_string())
            .bind("Title")
            .bind("Processing")
            .bind(job.id().to_string())
            .bind("en")
            .bind(project.created_at().to_rfc3339())
            .bind(project.updated_at().to_rfc3339())
            .bind(1)
            .execute(&pool).await.unwrap();

    sqlx::query("INSERT INTO jobs (id, project_id, kind, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(job.id().to_string())
            .bind(project.id().to_string())
            .bind("dubbing")
            .bind("running") // Status before crash
            .bind("{}")
            .bind(None::<String>)
            .bind(job.created_at().to_rfc3339())
            .bind(job.updated_at().to_rfc3339())
            .execute(&pool).await.unwrap();

    // Simulate CONFLICT: Project changed to something completely different
    sqlx::query("UPDATE projects SET status = 'Completed', active_job_id = NULL WHERE id = ?")
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
        expected_active_job_id: expected_active.clone(),
    };

    let err = commit_failed_interrupted_pair(&pool, cmd)
        .await
        .unwrap_err();
    assert!(matches!(err, ports::error::PortError::Conflict { .. }));
}
