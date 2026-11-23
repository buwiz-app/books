-- Schema 0026: a statement's activity feed (what a person did on it, for
-- its Activity Log) and the notes they left on it (its Comments tab).
-- One activity row per action, in order; a comment keeps only its body.
CREATE TABLE statement_activity (
  statement_id INTEGER NOT NULL REFERENCES statement (statement_id),
  ord          INTEGER NOT NULL,
  event        TEXT NOT NULL,
  detail       TEXT,
  at           TEXT NOT NULL,
  PRIMARY KEY (statement_id, ord)
);

CREATE TABLE statement_comment (
  comment_id   INTEGER PRIMARY KEY,
  statement_id INTEGER NOT NULL REFERENCES statement (statement_id),
  body         TEXT NOT NULL,
  at           TEXT NOT NULL
);
