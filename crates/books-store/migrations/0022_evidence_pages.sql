-- A PDF's pages as pictures (PNG), drawn once when the document is kept, so
-- the Books window can show each page of a PDF statement behind its lines.
-- They are renderings of the evidence, not evidence of their own.
CREATE TABLE evidence_page (
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  page INTEGER NOT NULL CHECK (page BETWEEN 1 AND 500),
  byte_length INTEGER NOT NULL,
  bytes BLOB NOT NULL,
  PRIMARY KEY (evidence_id, page)
);
