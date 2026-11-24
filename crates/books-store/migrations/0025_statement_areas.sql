-- Named areas of a statement page the assistant boxed: the title and the
-- statement date, for the window to highlight. One row per area, in order.
CREATE TABLE statement_draft_area (
  draft_id INTEGER NOT NULL REFERENCES statement_draft(draft_id),
  ord INTEGER NOT NULL,
  kind TEXT NOT NULL,
  label TEXT NOT NULL,
  page INTEGER NOT NULL CHECK (page BETWEEN 1 AND 500),
  bx0 INTEGER NOT NULL CHECK (bx0 BETWEEN 0 AND 1000),
  by0 INTEGER NOT NULL CHECK (by0 BETWEEN 0 AND 1000),
  bx1 INTEGER NOT NULL CHECK (bx1 BETWEEN 0 AND 1000),
  by1 INTEGER NOT NULL CHECK (by1 BETWEEN 0 AND 1000),
  CHECK (bx0 < bx1 AND by0 < by1),
  PRIMARY KEY (draft_id, ord)
);

CREATE TABLE statement_area (
  statement_id INTEGER NOT NULL REFERENCES statement(statement_id),
  ord INTEGER NOT NULL,
  kind TEXT NOT NULL,
  label TEXT NOT NULL,
  page INTEGER NOT NULL CHECK (page BETWEEN 1 AND 500),
  bx0 INTEGER NOT NULL CHECK (bx0 BETWEEN 0 AND 1000),
  by0 INTEGER NOT NULL CHECK (by0 BETWEEN 0 AND 1000),
  bx1 INTEGER NOT NULL CHECK (bx1 BETWEEN 0 AND 1000),
  by1 INTEGER NOT NULL CHECK (by1 BETWEEN 0 AND 1000),
  CHECK (bx0 < bx1 AND by0 < by1),
  PRIMARY KEY (statement_id, ord)
);
