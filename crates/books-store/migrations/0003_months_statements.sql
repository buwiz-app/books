-- Schema 0003 (M4): month locks and their events, bank statements and
-- their lines, the book's default categories, and recategorized entries.

-- A month that is closed refuses every post and void dated in it (DD-16).
-- A month with no row is open.
CREATE TABLE period (
  book_id    INTEGER NOT NULL REFERENCES book (book_id),
  period_key TEXT NOT NULL,
  status     TEXT NOT NULL CHECK (status IN ('open', 'closed')),
  closed_at  TEXT,
  PRIMARY KEY (book_id, period_key)
);

-- Every close and reopen, with its reason (no memo, no amount).
CREATE TABLE period_event (
  event_id   INTEGER PRIMARY KEY,
  book_id    INTEGER NOT NULL REFERENCES book (book_id),
  period_key TEXT NOT NULL,
  event      TEXT NOT NULL CHECK (event IN ('close', 'reopen')),
  reason     TEXT,
  at         TEXT NOT NULL
);

-- One bank or card statement for one account and month.
CREATE TABLE statement (
  statement_id INTEGER PRIMARY KEY,
  book_id      INTEGER NOT NULL REFERENCES book (book_id),
  account_id   INTEGER NOT NULL REFERENCES account (account_id),
  period_key   TEXT NOT NULL,
  closing      INTEGER NOT NULL,
  source_name  TEXT,
  status       TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'finished')),
  imported_at  TEXT NOT NULL,
  finished_at  TEXT,
  UNIQUE (book_id, account_id, period_key)
);

-- A statement line: money into the account is positive.
CREATE TABLE statement_line (
  line_id          INTEGER PRIMARY KEY,
  statement_id     INTEGER NOT NULL REFERENCES statement (statement_id),
  line_no          INTEGER NOT NULL,
  line_date        TEXT NOT NULL,
  description      TEXT NOT NULL,
  amount           INTEGER NOT NULL,
  matched_entry_id INTEGER REFERENCES entry (entry_id),
  match_kind       TEXT CHECK (match_kind IN ('exact', 'accepted', 'added')),
  UNIQUE (statement_id, line_no)
);

-- "Used when nothing else says": the Defaults tab of Settings.
CREATE TABLE category_default (
  book_id    INTEGER NOT NULL REFERENCES book (book_id),
  slot       TEXT NOT NULL,
  account_id INTEGER NOT NULL REFERENCES account (account_id),
  PRIMARY KEY (book_id, slot)
);

-- A posted line on a "not sure yet" account gets its category by a
-- reclassifying entry; the original then needs no review.
ALTER TABLE entry ADD COLUMN recategorized_by_entry_id INTEGER REFERENCES entry (entry_id);
ALTER TABLE entry ADD COLUMN recategorizes_entry_id INTEGER REFERENCES entry (entry_id);
