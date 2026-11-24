ALTER TABLE account ADD COLUMN parent_account_id INTEGER REFERENCES account (account_id);
CREATE INDEX account_by_parent ON account (book_id, parent_account_id);
