-- The audit rows of one entry, one owed item or one book: `pressed_at` and the
-- press card read them by subject, and the table grows with every write.
CREATE INDEX audit_by_subject ON audit_event (subject_kind, subject_id);
