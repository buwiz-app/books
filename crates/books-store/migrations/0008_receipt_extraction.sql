CREATE TABLE entry_extraction (
  id INTEGER PRIMARY KEY,
  entry_id INTEGER NOT NULL UNIQUE REFERENCES entry(entry_id),
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  fields_json TEXT NOT NULL,
  confidence_int INTEGER CHECK(confidence_int BETWEEN 0 AND 100),
  proposed_by TEXT NOT NULL CHECK(proposed_by IN ('app','agent','routine','unknown')),
  created_at TEXT NOT NULL
);
CREATE TABLE entry_extraction_correction (
  id INTEGER PRIMARY KEY,
  extraction_id INTEGER NOT NULL REFERENCES entry_extraction(id),
  field TEXT NOT NULL CHECK(field IN ('date','amount','category','person','reference')),
  value TEXT NOT NULL,
  corrected_by TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX entry_extraction_correction_by ON entry_extraction_correction(extraction_id, id);
