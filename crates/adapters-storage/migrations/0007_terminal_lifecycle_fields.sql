ALTER TABLE projects ADD COLUMN last_terminal_job_id TEXT;
ALTER TABLE outbox_messages ADD COLUMN deduplication_key TEXT;
CREATE UNIQUE INDEX idx_outbox_dedup_key ON outbox_messages(deduplication_key) WHERE deduplication_key IS NOT NULL;
