-- M10 parity reconcile-1: statements the assistant read, and where each line sits on its page.
ALTER TABLE statement ADD COLUMN evidence_id INTEGER REFERENCES evidence(id);
ALTER TABLE statement_line ADD COLUMN balance INTEGER;
ALTER TABLE statement_line ADD COLUMN page INTEGER CHECK (page IS NULL OR page BETWEEN 1 AND 500);
ALTER TABLE statement_line ADD COLUMN bx0 INTEGER CHECK (bx0 IS NULL OR bx0 BETWEEN 0 AND 1000);
ALTER TABLE statement_line ADD COLUMN by0 INTEGER CHECK (by0 IS NULL OR by0 BETWEEN 0 AND 1000);
ALTER TABLE statement_line ADD COLUMN bx1 INTEGER CHECK (bx1 IS NULL OR bx1 BETWEEN 0 AND 1000);
ALTER TABLE statement_line ADD COLUMN by1 INTEGER CHECK (by1 IS NULL OR by1 BETWEEN 0 AND 1000)
  CHECK ((bx0 IS NULL AND by0 IS NULL AND bx1 IS NULL AND by1 IS NULL)
      OR (bx0 IS NOT NULL AND by0 IS NOT NULL AND bx1 IS NOT NULL AND by1 IS NOT NULL AND bx0 < bx1 AND by0 < by1));
ALTER TABLE statement_line ADD COLUMN confidence INTEGER CHECK (confidence IS NULL OR confidence BETWEEN 0 AND 100);

CREATE TABLE statement_draft (
  draft_id INTEGER PRIMARY KEY,
  book_id INTEGER NOT NULL REFERENCES book(book_id),
  account_id INTEGER NOT NULL REFERENCES account(account_id),
  period_key TEXT NOT NULL,
  closing INTEGER,
  evidence_id INTEGER REFERENCES evidence(id),
  proposed_by TEXT NOT NULL CHECK (proposed_by IN ('agent','app')),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','accepted','dismissed','replaced')),
  created_at TEXT NOT NULL,
  decided_at TEXT
);
CREATE INDEX statement_draft_month ON statement_draft(book_id, account_id, period_key, status);

CREATE TABLE statement_draft_line (
  draft_id INTEGER NOT NULL REFERENCES statement_draft(draft_id),
  line_no INTEGER NOT NULL CHECK (line_no >= 1),
  line_date TEXT NOT NULL,
  description TEXT NOT NULL,
  amount INTEGER NOT NULL,
  balance INTEGER,
  page INTEGER CHECK (page IS NULL OR page BETWEEN 1 AND 500),
  bx0 INTEGER CHECK (bx0 IS NULL OR bx0 BETWEEN 0 AND 1000),
  by0 INTEGER CHECK (by0 IS NULL OR by0 BETWEEN 0 AND 1000),
  bx1 INTEGER CHECK (bx1 IS NULL OR bx1 BETWEEN 0 AND 1000),
  by1 INTEGER CHECK (by1 IS NULL OR by1 BETWEEN 0 AND 1000),
  confidence INTEGER CHECK (confidence IS NULL OR confidence BETWEEN 0 AND 100),
  CHECK ((bx0 IS NULL AND by0 IS NULL AND bx1 IS NULL AND by1 IS NULL)
      OR (bx0 IS NOT NULL AND by0 IS NOT NULL AND bx1 IS NOT NULL AND by1 IS NOT NULL AND bx0 < bx1 AND by0 < by1)),
  UNIQUE (draft_id, line_no)
);
