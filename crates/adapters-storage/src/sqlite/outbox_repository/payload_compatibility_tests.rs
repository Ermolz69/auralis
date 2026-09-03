use super::SqliteOutboxRepository;
use domain::outbox::{OutboxMessageId, OutboxPayload, WorkspaceKey};
use ports::repository::OutboxRepository;

#[test]
fn allocation_cleanup_serializes_with_allocation_name() {
    let payload = OutboxPayload::DeleteWorkspaceAllocation {
        workspace_key: WorkspaceKey::new("tmp/project/download_id").unwrap(),
    };
    assert_eq!(
        serde_json::to_value(payload).unwrap(),
        serde_json::json!({
            "type": "delete_workspace_allocation",
            "workspace_key": "tmp/project/download_id"
        })
    );
}

#[tokio::test]
async fn legacy_cleanup_payloads_remain_readable_without_inventing_keys() {
    let root = tempfile::tempdir().unwrap();
    let pool = crate::sqlite::connect_sqlite(root.path().join("test.db"))
        .await
        .unwrap();
    let repo = SqliteOutboxRepository::new(pool.clone());
    let keys = ["tmp/project/download_id", "original.mp4"];

    for key in keys {
        sqlx::query(
            "INSERT INTO outbox_messages
             (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at)
             VALUES (?, 'delete_workspace_file', ?, 'pending', 0,
                     '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
        )
        .bind(OutboxMessageId::new().to_string())
        .bind(
            serde_json::json!({ "type": "delete_workspace_file", "workspace_key": key })
                .to_string(),
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.corrupted_isolated, 0);
    assert_eq!(pending.isolation_errors, 0);
    assert_eq!(pending.messages.len(), keys.len());
    for key in keys {
        assert!(pending.messages.iter().any(|message| matches!(
            &message.payload,
            OutboxPayload::DeleteWorkspaceAllocation { workspace_key }
                if workspace_key.as_str() == key
        )));
    }
    pool.close().await;
}
