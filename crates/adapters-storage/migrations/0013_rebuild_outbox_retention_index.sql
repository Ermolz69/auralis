DROP INDEX IF EXISTS idx_outbox_status_created_at;
DROP INDEX IF EXISTS idx_outbox_retention;

UPDATE outbox_messages
SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', updated_at)
WHERE status IN ('done', 'dead')
  AND strftime('%Y-%m-%dT%H:%M:%SZ', updated_at) IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_outbox_retention
ON outbox_messages(status, updated_at, id);
