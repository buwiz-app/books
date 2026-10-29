-- Schema 0005 (M6): a certificate's status as it changes. A 2307 is a
-- posted line (document_kind certificate_2307) and a posted line never
-- changes, so receiving the paper later, or losing it, is a row here; the
-- latest row is the status the bridge reads. Only received ones count.

CREATE TABLE certificate_status (
  line_id    INTEGER NOT NULL REFERENCES line (line_id),
  status     TEXT NOT NULL CHECK (status IN ('received', 'pending', 'lost', 'disputed')),
  changed_at TEXT NOT NULL,
  seq        INTEGER PRIMARY KEY
);
CREATE INDEX certificate_status_line ON certificate_status (line_id, seq);
