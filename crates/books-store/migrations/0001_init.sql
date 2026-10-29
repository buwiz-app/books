-- Schema 0001: a book, its chart, entries and lines, idempotency, audit
-- and tickets. The whole file is SQLCipher-encrypted; there is no TIN
-- column anywhere (invariant 4). Money is INTEGER centavos.

CREATE TABLE meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE book (
  book_id              INTEGER PRIMARY KEY,
  label                TEXT NOT NULL,
  country              TEXT NOT NULL DEFAULT 'PH',
  basis                TEXT NOT NULL DEFAULT 'cash' CHECK (basis IN ('cash', 'accrual')),
  fiscal_year_start    INTEGER NOT NULL DEFAULT 1 CHECK (fiscal_year_start BETWEEN 1 AND 12),
  coa_preset           TEXT NOT NULL,
  coa_preset_version   INTEGER NOT NULL,
  -- the opaque buwiz-cloud tax-profile id (ADR 0003); never a TIN
  cloud_tax_profile_id TEXT,
  created_at           TEXT NOT NULL
);

CREATE TABLE account (
  account_id     INTEGER PRIMARY KEY,
  book_id        INTEGER NOT NULL REFERENCES book (book_id),
  number         TEXT NOT NULL,
  name           TEXT NOT NULL,
  type           TEXT NOT NULL CHECK (type IN ('asset', 'liability', 'equity', 'revenue',
                   'cost_of_revenue', 'expense', 'other_income', 'other_expense')),
  -- a kind id of data/categories.v1.json, 'mixed' or 'unsure'
  kind           TEXT NOT NULL,
  -- 0 while an imported account's match waits for the owner (M5)
  kind_confirmed INTEGER NOT NULL DEFAULT 1 CHECK (kind_confirmed IN (0, 1)),
  active         INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  UNIQUE (book_id, number)
);

CREATE TABLE entry (
  entry_id           INTEGER PRIMARY KEY,
  book_id            INTEGER NOT NULL REFERENCES book (book_id),
  -- shown as 0187
  entry_no           INTEGER NOT NULL,
  entry_date         TEXT NOT NULL,
  status             TEXT NOT NULL CHECK (status IN ('draft', 'posted', 'voided', 'discarded')),
  form               TEXT NOT NULL CHECK (form IN ('money_in', 'money_out', 'transfer', 'journal', 'reversal')),
  payee              TEXT,
  memo               TEXT,
  reference          TEXT,
  reverses_entry_id  INTEGER REFERENCES entry (entry_id),
  voided_by_entry_id INTEGER REFERENCES entry (entry_id),
  origin             TEXT NOT NULL CHECK (origin IN ('app', 'agent', 'routine', 'import')),
  needs_review       INTEGER NOT NULL DEFAULT 0 CHECK (needs_review IN (0, 1)),
  created_at         TEXT NOT NULL,
  posted_at          TEXT,
  UNIQUE (book_id, entry_no)
);
CREATE INDEX entry_by_date ON entry (book_id, entry_date);

CREATE TABLE line (
  line_id    INTEGER PRIMARY KEY,
  entry_id   INTEGER NOT NULL REFERENCES entry (entry_id),
  line_no    INTEGER NOT NULL,
  account_id INTEGER NOT NULL REFERENCES account (account_id),
  debit      INTEGER NOT NULL CHECK (debit >= 0),
  credit     INTEGER NOT NULL CHECK (credit >= 0),
  -- the line's own facts (JSON); the account's kind supplies the rest
  facts      TEXT NOT NULL DEFAULT '{}',
  UNIQUE (entry_id, line_no)
);
CREATE INDEX line_by_account ON line (account_id);

CREATE TABLE idempotency (
  client_request_id TEXT PRIMARY KEY,
  tool              TEXT NOT NULL,
  request_sha256    TEXT NOT NULL,
  result            TEXT NOT NULL,
  created_at        TEXT NOT NULL
);

-- No memo, no TIN and no amount (docs/01 6.5)
CREATE TABLE audit_event (
  event_id     INTEGER PRIMARY KEY,
  at           TEXT NOT NULL,
  origin       TEXT NOT NULL,
  tool         TEXT NOT NULL,
  subject_kind TEXT,
  subject_id   INTEGER,
  outcome      TEXT NOT NULL
);

CREATE TABLE ticket (
  ticket       TEXT PRIMARY KEY,
  subject_kind TEXT NOT NULL,
  subject_id   INTEGER NOT NULL,
  created_at   TEXT NOT NULL,
  revoked_at   TEXT,
  UNIQUE (subject_kind, subject_id)
);
