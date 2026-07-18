#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::setup_db;
use crate::sqlite::unit_of_work::SqliteStorageUnitOfWork;

use domain::media::{Artifact, ArtifactId};
use domain::project::Project;
use ports::error::PortError;
use ports::transaction::{CommitManagedSourceImport, CommitProjectDelete, StorageUnitOfWork};

#[tokio::test]
async fn test_commit_managed_source_import_writes_atomically() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());

    let mut project = Project::new("Tx Test".to_string());
    let original_updated_at = project.updated_at();

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(project.id().to_string())
    .bind(project.title())
    .bind("Draft")
    .bind(project.created_at().to_rfc3339())
    .bind(original_updated_at.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let artifact_id = ArtifactId::new();
    let final_key = format!("{}/source-video/{}.mp4", project.id(), artifact_id);
    let staging_key = format!(".staging/uuid/{}.mp4", artifact_id);

    let artifact = Artifact {
        id: artifact_id.clone(),
        kind: domain::media::ArtifactKind::SourceVideo,
        location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
        size_bytes: Some(1024),
        state: domain::media::ArtifactState::PendingFinalize,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ready_at: None,
    };

    project
        .import_source(
            domain::media::MediaSource::ManagedLocalFile {
                artifact_id: artifact.id.clone(),
                original_filename: "test.mp4".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();

    let cmd = CommitManagedSourceImport {
        project: project.clone(),
        artifact: artifact.clone(),
        staging_key: staging_key.clone(),
        final_key: final_key.clone(),
        original_updated_at,
    };

    uow.commit_managed_source_import(cmd).await.unwrap();

    let project_row: crate::sqlite::project_row::ProjectRow =
        sqlx::query_as("SELECT * FROM projects WHERE id = ?")
            .bind(project.id().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(project_row.status, "ReadyForProcessing");

    let artifact_row: crate::sqlite::artifact_index::row::ArtifactRow =
        sqlx::query_as("SELECT * FROM artifacts WHERE id = ?")
            .bind(artifact.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(artifact_row.state, "pending_finalize");
    assert_eq!(artifact_row.project_id, project.id().to_string());

    let outbox_rows: Vec<crate::sqlite::outbox_row::OutboxRow> =
        sqlx::query_as("SELECT * FROM outbox_messages")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(outbox_rows.len(), 1);

    let jobs_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(jobs_count, 0);
}

#[tokio::test]
async fn test_commit_managed_source_import_missing_project_returns_not_found() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());

    let mut project = Project::new("Missing".to_string());
    let artifact_id = ArtifactId::new();
    let final_key = format!("{}/source-video/{}.mp4", project.id(), artifact_id);
    let staging_key = format!(".staging/uuid/{}.mp4", artifact_id);

    let artifact = Artifact {
        id: artifact_id.clone(),
        kind: domain::media::ArtifactKind::SourceVideo,
        location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
        size_bytes: Some(1024),
        state: domain::media::ArtifactState::PendingFinalize,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ready_at: None,
    };

    let original_updated_at = project.updated_at();
    project
        .import_source(
            domain::media::MediaSource::ManagedLocalFile {
                artifact_id: artifact.id.clone(),
                original_filename: "test.mp4".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();

    let cmd = CommitManagedSourceImport {
        project,
        artifact,
        staging_key,
        final_key,
        original_updated_at,
    };

    let err = uow.commit_managed_source_import(cmd).await.unwrap_err();
    assert!(matches!(err, PortError::NotFound { .. }));

    let artifacts_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(artifacts_count, 0);
}

#[tokio::test]
async fn test_commit_managed_source_import_status_mismatch_returns_conflict() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());

    let mut project = Project::new("Status Mismatch".to_string());
    let original_updated_at = project.updated_at();

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(project.id().to_string())
    .bind(project.title())
    .bind("ReadyForProcessing") // DB status is already ReadyForProcessing, not Draft!
    .bind(project.created_at().to_rfc3339())
    .bind(original_updated_at.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let artifact_id = ArtifactId::new();
    let final_key = format!("{}/source-video/{}.mp4", project.id(), artifact_id);
    let staging_key = format!(".staging/uuid/{}.mp4", artifact_id);

    let artifact = Artifact {
        id: artifact_id.clone(),
        kind: domain::media::ArtifactKind::SourceVideo,
        location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
        size_bytes: Some(1024),
        state: domain::media::ArtifactState::PendingFinalize,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        ready_at: None,
    };

    project
        .import_source(
            domain::media::MediaSource::ManagedLocalFile {
                artifact_id: artifact.id.clone(),
                original_filename: "test.mp4".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();

    let cmd = CommitManagedSourceImport {
        project,
        artifact,
        staging_key,
        final_key,
        original_updated_at,
    };

    let err = uow.commit_managed_source_import(cmd).await.unwrap_err();
    assert!(matches!(err, PortError::Conflict { .. }));
}

#[tokio::test]
async fn test_commit_project_delete_rolls_back_on_invalid_job_id() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = domain::project::ProjectId::new();

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(project_id.to_string())
    .bind("Corrupt Job Test")
    .bind("Draft")
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO jobs (id, project_id, kind, title, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind("invalid-job-uuid-12345")
        .bind(project_id.to_string())
        .bind("Extracting")
        .bind("Job title")
        .bind("Pending")
        .bind(r#"{}"#)
        .bind::<Option<String>>(None)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();

    let cmd = CommitProjectDelete {
        project_id: project_id.clone(),
    };

    let result = uow.commit_project_delete(cmd).await;

    match result {
        Err(PortError::InvalidStoredData { field, .. }) => {
            assert_eq!(field, "id");
        }
        Ok(_) => panic!("Expected InvalidStoredData error, got Ok"),
        Err(e) => panic!("Expected InvalidStoredData error, got Err({:?})", e),
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(project_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count, 0);
}

#[tokio::test]
async fn test_commit_project_delete_conflict_when_missing() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = domain::project::ProjectId::new();

    let cmd = CommitProjectDelete {
        project_id: project_id.clone(),
    };

    let result = uow.commit_project_delete(cmd).await;

    match result {
        Err(PortError::NotFound { .. }) => {}
        Ok(_) => panic!("Expected NotFound on missing project initial check, got Ok"),
        Err(e) => panic!(
            "Expected NotFound on missing project initial check, got Err({:?})",
            e
        ),
    }
}

#[sqlx::test]
async fn test_commit_project_delete_busy_lock_contention(pool: sqlx::SqlitePool) {
    let project_id = domain::project::ProjectId::new();
    sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'Test', 'draft', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let mut blocker_tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET title = 'Locked' WHERE id = ?")
        .bind(project_id.to_string())
        .execute(&mut *blocker_tx)
        .await
        .unwrap();

    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let cmd = CommitProjectDelete {
        project_id: project_id.clone(),
    };

    let result = uow.commit_project_delete(cmd).await;

    match result {
        Err(PortError::Busy { .. }) => {}
        Err(e) => panic!("Expected Busy, got {:?}", e),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}
