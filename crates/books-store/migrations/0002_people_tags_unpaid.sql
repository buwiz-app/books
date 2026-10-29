-- Schema 0002 (M2): people, tags and unpaid documents. No TIN column:
-- the BIR app keeps every TIN (invariant 4). A person is a name and a kind.

CREATE TABLE person (
  person_id  INTEGER PRIMARY KEY,
  book_id    INTEGER NOT NULL REFERENCES book (book_id),
  kind       TEXT NOT NULL CHECK (kind IN ('vendor', 'customer', 'employee', 'owner', 'lender', 'government')),
  name       TEXT NOT NULL,
  note       TEXT,
  active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  UNIQUE (book_id, kind, name)
);

CREATE TABLE tag (
  tag_id     INTEGER PRIMARY KEY,
  book_id    INTEGER NOT NULL REFERENCES book (book_id),
  kind       TEXT NOT NULL CHECK (kind IN ('department', 'location')),
  name       TEXT NOT NULL,
  active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  UNIQUE (book_id, kind, name)
);

ALTER TABLE entry ADD COLUMN person_id INTEGER REFERENCES person (person_id);
ALTER TABLE entry ADD COLUMN department_id INTEGER REFERENCES tag (tag_id);
ALTER TABLE entry ADD COLUMN location_id INTEGER REFERENCES tag (tag_id);
CREATE INDEX entry_by_person ON entry (person_id);

-- An invoice to collect or a bill to pay. On a cash book it is a reminder,
-- not an entry: marking it paid posts the money on the day it moves
-- (ADR 0008). On an accrual book it posts the receivable or payable when
-- it is saved, and marking it paid clears it.
CREATE TABLE unpaid (
  unpaid_id           INTEGER PRIMARY KEY,
  book_id             INTEGER NOT NULL REFERENCES book (book_id),
  side                TEXT NOT NULL CHECK (side IN ('collect', 'pay')),
  person_id           INTEGER REFERENCES person (person_id),
  reference           TEXT,
  memo                TEXT,
  issued              TEXT NOT NULL,
  due                 TEXT,
  amount              INTEGER NOT NULL CHECK (amount > 0),
  category_account_id INTEGER NOT NULL REFERENCES account (account_id),
  status              TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'paid', 'cancelled')),
  accrual_entry_id    INTEGER REFERENCES entry (entry_id),
  paid_entry_id       INTEGER REFERENCES entry (entry_id),
  paid_on             TEXT,
  created_at          TEXT NOT NULL
);
CREATE INDEX unpaid_open ON unpaid (book_id, side, status);
