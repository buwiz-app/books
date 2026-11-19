-- One picture for each page of a statement the assistant read. A statement
-- has one evidence document (statement.evidence_id); a statement read from
-- several pictures names one evidence for each page here.
CREATE TABLE statement_draft_page (
  draft_id INTEGER NOT NULL REFERENCES statement_draft(draft_id),
  page INTEGER NOT NULL CHECK (page BETWEEN 1 AND 500),
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  PRIMARY KEY (draft_id, page)
);

CREATE TABLE statement_page (
  statement_id INTEGER NOT NULL REFERENCES statement(statement_id),
  page INTEGER NOT NULL CHECK (page BETWEEN 1 AND 500),
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  PRIMARY KEY (statement_id, page)
);
