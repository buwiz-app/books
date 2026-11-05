-- What the person's card showed, by its hash: the record of a press says what
-- they read as well as what went out.
ALTER TABLE audit_event ADD COLUMN press_shown_hash TEXT;
