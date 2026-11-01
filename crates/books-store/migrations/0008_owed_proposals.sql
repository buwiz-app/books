CREATE TABLE owed_proposal (
  id INTEGER PRIMARY KEY,
  book_id INTEGER NOT NULL REFERENCES book(book_id),
  evidence_id INTEGER NOT NULL REFERENCES evidence(id),
  side TEXT NOT NULL CHECK(side IN ('collect','pay')),
  issued TEXT NOT NULL,
  due TEXT,
  amount_centavos INTEGER NOT NULL CHECK(amount_centavos > 0),
  category_id INTEGER REFERENCES account(account_id),
  person_suggestion_id INTEGER REFERENCES person(person_id),
  confidence_int INTEGER NOT NULL CHECK(confidence_int BETWEEN 0 AND 100),
  fields_json TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','accepted','discarded')),
  accepted_owed_id INTEGER REFERENCES unpaid(unpaid_id),
  created_at TEXT NOT NULL,
  CHECK((status = 'accepted') = (accepted_owed_id IS NOT NULL))
);
CREATE INDEX owed_proposal_pending ON owed_proposal(book_id, side, status);
CREATE UNIQUE INDEX owed_proposal_accepted ON owed_proposal(accepted_owed_id) WHERE accepted_owed_id IS NOT NULL;
