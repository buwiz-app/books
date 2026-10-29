-- Schema 0004 (M5): where an account came from and its suggested kind
-- (Map 1), opening totals with proof for a mid-year start, the owner's OK
-- of a return event, and the import log.

ALTER TABLE account ADD COLUMN source TEXT NOT NULL DEFAULT 'preset' CHECK (source IN ('preset', 'user', 'import'));
-- the kind Map 1 suggested and how sure (0 to 100); the owner confirms
ALTER TABLE account ADD COLUMN kind_suggested TEXT;
ALTER TABLE account ADD COLUMN confidence INTEGER;
-- the old books' name, when the imported account was matched to a preset one
ALTER TABLE account ADD COLUMN imported_name TEXT;

-- January to now for a book that starts mid-year: one measure's total
-- over a range, with what proves it (owner decision: opening totals with
-- proof). Lineage shows it as an opening figure.
CREATE TABLE opening_total (
  book_id     INTEGER NOT NULL REFERENCES book (book_id),
  measure     TEXT NOT NULL,
  period_from TEXT NOT NULL,
  period_to   TEXT NOT NULL,
  centavos    INTEGER NOT NULL,
  proof       TEXT NOT NULL,
  entered_at  TEXT NOT NULL,
  PRIMARY KEY (book_id, measure, period_from)
);

-- The owner's OK of a return event (a credit note or a later-period
-- reversal) before a proposal uses it (GATE.RETURNS_ACK).
CREATE TABLE return_ack (
  book_id         INTEGER NOT NULL REFERENCES book (book_id),
  entry_id        INTEGER NOT NULL REFERENCES entry (entry_id),
  acknowledged_at TEXT NOT NULL,
  PRIMARY KEY (book_id, entry_id)
);

CREATE TABLE import_batch (
  batch_id    INTEGER PRIMARY KEY,
  book_id     INTEGER NOT NULL REFERENCES book (book_id),
  source      TEXT NOT NULL,
  file_name   TEXT,
  accounts    INTEGER NOT NULL,
  entries     INTEGER NOT NULL,
  skipped     TEXT NOT NULL,
  imported_at TEXT NOT NULL
);
ALTER TABLE entry ADD COLUMN import_batch_id INTEGER REFERENCES import_batch (batch_id);
