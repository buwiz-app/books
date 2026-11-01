-- Schema 0012 (M10 parity): every finish and reopen of a bank statement.
-- A reopen keeps its reason (it may be a memo, so no result echoes it).
CREATE TABLE statement_event (
  event_id     INTEGER PRIMARY KEY,
  statement_id INTEGER NOT NULL REFERENCES statement (statement_id),
  event        TEXT NOT NULL CHECK (event IN ('finish', 'reopen')),
  reason       TEXT,
  at           TEXT NOT NULL
);
