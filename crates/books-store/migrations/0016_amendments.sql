-- Amend by reversal: the original stays voided, a posted reversal cancels
-- it, and a draft replacement carries the corrected entry. The reason is
-- kept here only; no tool output, title, card or log carries it.
ALTER TABLE entry ADD COLUMN amends_entry_id INTEGER REFERENCES entry (entry_id);
ALTER TABLE entry ADD COLUMN amended_by_entry_id INTEGER REFERENCES entry (entry_id);

CREATE TABLE amendment (
  original_entry_id    INTEGER PRIMARY KEY REFERENCES entry (entry_id),
  reversal_entry_id    INTEGER NOT NULL REFERENCES entry (entry_id),
  replacement_entry_id INTEGER NOT NULL REFERENCES entry (entry_id),
  reason               TEXT NOT NULL CHECK (length(reason) BETWEEN 3 AND 200),
  created_at           TEXT NOT NULL
);
