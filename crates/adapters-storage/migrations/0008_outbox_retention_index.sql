CREATE INDEX idx_outbox_status_created_at
ON outbox_messages(status, created_at);
