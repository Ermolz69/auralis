#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::setup_db;
use crate::sqlite::unit_of_work::SqliteStorageUnitOfWork;

use domain::media::ArtifactId;
use domain::outbox::OutboxPayload;
use domain::project::ProjectId;
use ports::transaction::{CommitProjectDelete, StorageUnitOfWork};

#[tokio::test]
async fn test_project_delete_order_and_filtering() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Job belonging to project
    sqlx::query("INSERT INTO jobs (id, project_id, kind, title, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, 'Extracting', 'J', 'Pending', '{}', NULL, 'now', 'now')")
        .bind(domain::job::JobId::new().to_string())
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Artifact A: StorageKey
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'StorageKey', 'project/art-A', 'ready', 'now', 'now')")
        .bind(ArtifactId::new().to_string())
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Artifact B: LocalPath (should NOT create outbox delete payload)
    sqlx::query("INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', 'project/art-B', 'ready', 'now', 'now')")
        .bind(ArtifactId::new().to_string())
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Old project message to cancel
    let old_msg_id = domain::outbox::OutboxMessageId::new();
    sqlx::query("INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at, aggregate_type, aggregate_id) VALUES (?, 'finalize_staged_artifact', '{}', 'pending', 0, 'now', 'now', 'now', 'project', ?)")
        .bind(old_msg_id.to_string())
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    let cmd = CommitProjectDelete {
        project_id: project_id.clone(),
    };

    let result = uow.commit_project_delete(cmd).await.unwrap();
    assert_eq!(result.deleted_job_ids.len(), 1);

    // Verify database cascade
    let p_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(project_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(p_count, 0);

    let j_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(j_count, 0);

    let a_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(a_count, 0);

    // Verify outbox messages:
    // - The old message should be 'dead'
    let old_msg_status: String =
        sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
            .bind(old_msg_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(old_msg_status, "dead");

    // - There should be exactly 2 new messages in status 'pending':
    //   1. DeleteStorageKey for 'project/art-A'
    //   2. DeleteProjectArtifactDir for project_id
    let new_messages: Vec<crate::sqlite::outbox_row::OutboxRow> =
        sqlx::query_as("SELECT * FROM outbox_messages WHERE status = 'pending'")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(new_messages.len(), 2);

    let mut has_del_key = false;
    let mut has_del_dir = false;

    for msg in new_messages {
        let payload: OutboxPayload = serde_json::from_str(&msg.payload_json).unwrap();
        match payload {
            OutboxPayload::DeleteStorageKey { storage_key } => {
                assert_eq!(storage_key.to_string(), "project/art-A");
                has_del_key = true;
            }
            OutboxPayload::DeleteProjectArtifactDir {
                project_id: del_pid,
            } => {
                assert_eq!(del_pid, project_id);
                has_del_dir = true;
            }
            _ => panic!("Unexpected outbox payload"),
        }
    }

    assert!(has_del_key);
    assert!(has_del_dir);
}

#[tokio::test]
async fn test_project_delete_injected_failure_rolls_back() {
    let pool = setup_db().await;
    let uow = SqliteStorageUnitOfWork::new(pool.clone());
    let project_id = ProjectId::new();

    sqlx::query(
        "INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, 'P', 'draft', 'now', 'now')",
    )
    .bind(project_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Old project message to cancel
    let old_msg_id = domain::outbox::OutboxMessageId::new();
    sqlx::query("INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at, aggregate_type, aggregate_id) VALUES (?, 'finalize_staged_artifact', '{}', 'pending', 0, 'now', 'now', 'now', 'project', ?)")
        .bind(old_msg_id.to_string())
        .bind(project_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Create a trigger that aborts insertion into outbox_messages
    sqlx::query(
        "CREATE TRIGGER fail_outbox_insert BEFORE INSERT ON outbox_messages
         BEGIN
             SELECT RAISE(ABORT, 'Injected outbox failure');
         END;",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cmd = CommitProjectDelete {
        project_id: project_id.clone(),
    };

    let result = uow.commit_project_delete(cmd).await;
    assert!(result.is_err());

    // Drop the trigger so we can check DB
    sqlx::query("DROP TRIGGER fail_outbox_insert")
        .execute(&pool)
        .await
        .unwrap();

    // Verify project still exists (rolled back!)
    let p_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
        .bind(project_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(p_count, 1);

    // Verify old outbox message is still 'pending' (rolled back!)
    let old_msg_status: String =
        sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
            .bind(old_msg_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(old_msg_status, "pending");
}
