#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::setup_db;
use crate::sqlite::unit_of_work::SqliteStorageUnitOfWork;

use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::transaction::{CommitArtifactFinalize, StorageUnitOfWork};
use sqlx::Row;

#[tokio::test]
async fn test_finalize_pending_to_ready_and_check_constraint() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();
    let artifact_id = ArtifactId::new();
    let message_id = domain::outbox::OutboxMessageId::new();
    let ready_key = "final_storage_key";

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'temp_path', 'pending_finalize', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'finalize_staged_artifact', '{}', 'processing', 0, 'now', 'now', 'now')",
    )
    .bind(message_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };

    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Committed
    ));

    let row =
        sqlx::query("SELECT location_kind, location_value, state FROM artifacts WHERE id = ?")
            .bind(artifact_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.get::<String, _>("location_kind"), "StorageKey");
    assert_eq!(row.get::<String, _>("location_value"), ready_key);
    assert_eq!(row.get::<String, _>("state"), "ready");
}

#[tokio::test]
async fn test_finalize_retry_idempotency() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();
    let artifact_id = ArtifactId::new();
    let message_id = domain::outbox::OutboxMessageId::new();
    let ready_key = "final_storage_key";

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'StorageKey', ?, 'ready', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .bind(ready_key)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'finalize_staged_artifact', '{}', 'done', 0, 'now', 'now', 'now')",
    )
    .bind(message_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };

    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::AlreadyFinalized
    ));
}

#[tokio::test]
async fn test_finalize_conflict_cases() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();
    let artifact_id = ArtifactId::new();
    let message_id = domain::outbox::OutboxMessageId::new();
    let ready_key = "final_storage_key";

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'finalize_staged_artifact', '{}', 'processing', 0, 'now', 'now', 'now')",
    )
    .bind(message_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // 1. Ready with different key -> Conflict
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'StorageKey', 'diff_key', 'ready', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };
    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Conflict
    ));

    // Cleanup artifact
    sqlx::query("DELETE FROM artifacts")
        .execute(&pool)
        .await
        .unwrap();

    // 2. Ready with LocalPath -> Conflict
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'local_path', 'ready', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };
    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Conflict
    ));

    // Cleanup artifact
    sqlx::query("DELETE FROM artifacts")
        .execute(&pool)
        .await
        .unwrap();

    // 3. Wrong state (deleting) -> Conflict
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'local_path', 'deleting', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };
    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Conflict
    ));

    // Cleanup artifact
    sqlx::query("DELETE FROM artifacts")
        .execute(&pool)
        .await
        .unwrap();

    // 4. Artifact of another project -> Conflict
    let other_project_id = ProjectId::new();
    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P2', 'draft', 'now', 'now')",
    )
    .bind(other_project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'local_path', 'pending_finalize', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(other_project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };
    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Conflict
    ));

    // Cleanup artifact
    sqlx::query("DELETE FROM artifacts")
        .execute(&pool)
        .await
        .unwrap();

    // 5. Missing artifact, existing project -> Conflict
    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };
    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::Conflict
    ));
}

#[tokio::test]
async fn test_finalize_project_deleted_or_outbox_dead() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();
    let artifact_id = ArtifactId::new();
    let message_id = domain::outbox::OutboxMessageId::new();
    let ready_key = "final_storage_key";

    // Project exists, but outbox status is 'dead'
    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'temp_path', 'pending_finalize', 'now', 'now')",
    )
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'finalize_staged_artifact', '{}', 'dead', 0, 'now', 'now', 'now')",
    )
    .bind(message_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };

    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::ObsoleteBecauseProjectDeleted
    ));

    // Outbox exists and is processing, but project is deleted
    sqlx::query("DELETE FROM projects")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE outbox_messages SET status = 'processing'")
        .execute(&pool)
        .await
        .unwrap();

    let cmd = CommitArtifactFinalize {
        message_id: message_id.clone(),
        project_id: project_id.clone(),
        artifact_id: artifact_id.clone(),
        ready_key: ready_key.to_string(),
    };

    let result = uow.commit_artifact_finalize(cmd).await.unwrap();
    assert!(matches!(
        result,
        ports::transaction::CommitArtifactFinalizeResult::ObsoleteBecauseProjectDeleted
    ));
}
