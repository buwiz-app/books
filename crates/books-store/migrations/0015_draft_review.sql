-- Migration 15 (M10 parity, ledger-1): what the assistant proposed on a
-- draft, field by field, and the likely duplicates the user chose to keep.
-- Confidence is an integer percent the Bot's assistant reports; Books never
-- computes one.

CREATE TABLE draft_field (
  entry_id   INTEGER NOT NULL REFERENCES entry (entry_id),
  field      TEXT NOT NULL CHECK (field IN ('date', 'amount', 'account', 'to_account', 'category', 'payee', 'memo', 'reference', 'lines')),
  confidence INTEGER CHECK (confidence BETWEEN 0 AND 100),
  changed_by TEXT CHECK (changed_by IN ('agent', 'routine', 'app')),
  changed_at TEXT,
  PRIMARY KEY (entry_id, field)
);

-- "Keep both": the user posted this entry knowing it looks like the other
CREATE TABLE duplicate_ack (
  entry_id       INTEGER NOT NULL REFERENCES entry (entry_id),
  other_entry_id INTEGER NOT NULL REFERENCES entry (entry_id),
  acked_at       TEXT NOT NULL,
  PRIMARY KEY (entry_id, other_entry_id)
);

-- a draft started from another entry (books_entry_draft copy_of) is never
-- its source's duplicate
CREATE TABLE entry_copy (
  entry_id        INTEGER PRIMARY KEY REFERENCES entry (entry_id),
  source_entry_id INTEGER NOT NULL REFERENCES entry (entry_id)
);
