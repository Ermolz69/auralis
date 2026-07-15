ALTER TABLE projects ADD COLUMN last_terminal_job_id TEXT;
ALTER TABLE outbox_messages ADD COLUMN deduplication_key TEXT UNIQUE;

