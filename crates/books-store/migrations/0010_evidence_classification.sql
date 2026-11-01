CREATE TABLE evidence_classification (
  id INTEGER PRIMARY KEY,
  book_id INTEGER NOT NULL REFERENCES book(book_id),
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  proposed_kind TEXT NOT NULL CHECK(proposed_kind IN ('statement','bill','invoice','receipt','payslip','contract','tax_form','other')),
  confidence_int INTEGER NOT NULL CHECK(confidence_int BETWEEN 0 AND 100),
  status TEXT NOT NULL CHECK(status IN ('draft','accepted','discarded')),
  accepted_kind TEXT CHECK(accepted_kind IN ('statement','bill','invoice','receipt','payslip','contract','tax_form','other')),
  previous_kind TEXT,
  created_at TEXT NOT NULL,
  decided_at TEXT
);
CREATE INDEX evidence_classification_evidence ON evidence_classification(evidence_id);
