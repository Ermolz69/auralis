DROP INDEX IF EXISTS idx_outbox_status_created_at;
DROP INDEX IF EXISTS idx_outbox_retention;

CREATE INDEX idx_outbox_retention ON outbox_messages(status, updated_at, id);
