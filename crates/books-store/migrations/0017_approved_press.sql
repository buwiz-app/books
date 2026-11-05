-- Whose press a write was, when the AI pressed it and a person allowed that
-- one press. Handles and hashes only: never what the card showed.
ALTER TABLE audit_event ADD COLUMN press_who TEXT;
ALTER TABLE audit_event ADD COLUMN press_approved_by TEXT;
ALTER TABLE audit_event ADD COLUMN press_at INTEGER;
ALTER TABLE audit_event ADD COLUMN press_once TEXT;
ALTER TABLE audit_event ADD COLUMN press_args_hash TEXT;
