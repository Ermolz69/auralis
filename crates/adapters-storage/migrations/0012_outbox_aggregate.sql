ALTER TABLE outbox_messages ADD COLUMN aggregate_type TEXT;
ALTER TABLE outbox_messages ADD COLUMN aggregate_id TEXT;

CREATE INDEX idx_outbox_aggregate_status ON outbox_messages(aggregate_type, aggregate_id, status);

UPDATE outbox_messages 
SET aggregate_type = 'project', 
    aggregate_id = CASE WHEN json_valid(payload_json) THEN json_extract(payload_json, '$.project_id') ELSE NULL END
WHERE CASE WHEN json_valid(payload_json) THEN json_extract(payload_json, '$.project_id') IS NOT NULL ELSE 0 END;
