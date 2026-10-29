-- Schema 6 (M7): a hand-off of one form's Books totals to the BIR draft.
-- The app's click makes it and attaches it to the chat; the assistant
-- fills the BIR draft with source `document` (D3); the owner's click in
-- Books ("BIR shows these values") records the digest of what was handed
-- (B5). A later change to the inputs makes it out of date; a refresh
-- makes a new one and the old one is superseded. No TIN, no BIR ticket.

CREATE TABLE handoff (
  handoff_id       INTEGER PRIMARY KEY,
  book_id          INTEGER NOT NULL REFERENCES book (book_id),
  form             TEXT NOT NULL CHECK (form IN ('2551Q', '1601C', '1701MS')),
  period_key       TEXT NOT NULL,
  -- the BIR choices read from the BIR draft when it was made (JSON)
  form_context     TEXT NOT NULL,
  -- the input boxes and their values, exactly as handed (JSON)
  fill_args        TEXT NOT NULL,
  -- the proposal's inputs hash: a later change makes it differ
  inputs_hash      TEXT NOT NULL,
  -- each box's lineage lines with their amounts (JSON), to say what changed
  lineage          TEXT NOT NULL,
  source           TEXT NOT NULL DEFAULT 'document' CHECK (source = 'document'),
  status           TEXT NOT NULL CHECK (status IN ('handed', 'confirmed', 'superseded')),
  created_at       TEXT NOT NULL,
  fill_args_sha256 TEXT,
  confirmed_at     TEXT
);
CREATE INDEX handoff_by_form ON handoff (book_id, form, period_key, status);
