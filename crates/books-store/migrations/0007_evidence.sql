CREATE TABLE evidence (
  id INTEGER PRIMARY KEY,
  book_id INTEGER NOT NULL REFERENCES book(book_id),
  sha256 TEXT NOT NULL,
  mime TEXT NOT NULL,
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  bytes BLOB NOT NULL,
  page_count INTEGER CHECK(page_count BETWEEN 1 AND 100000),
  source TEXT NOT NULL CHECK(source IN ('user','assistant')),
  kind TEXT NOT NULL CHECK(kind IN ('statement','bill','invoice','receipt','payslip','contract','tax_form','other')),
  created_at TEXT NOT NULL,
  UNIQUE(book_id, sha256)
);
CREATE TABLE evidence_link (
  id INTEGER PRIMARY KEY,
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  entry_id INTEGER REFERENCES entry(entry_id),
  owed_id INTEGER REFERENCES unpaid(unpaid_id),
  created_at TEXT NOT NULL,
  CHECK((entry_id IS NOT NULL) + (owed_id IS NOT NULL) = 1)
);
CREATE UNIQUE INDEX evidence_entry ON evidence_link(evidence_id, entry_id) WHERE entry_id IS NOT NULL;
CREATE UNIQUE INDEX evidence_owed ON evidence_link(evidence_id, owed_id) WHERE owed_id IS NOT NULL;
