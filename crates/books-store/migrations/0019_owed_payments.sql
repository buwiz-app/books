-- Schema 0019 (M10 parity, unpaid-1): one row per settlement of an unpaid
-- document, linked to the entry that moved the money. What is paid to date
-- is the sum of rows whose entry is still posted, so voiding a payment in
-- the Ledger restores the balance with no extra write.

CREATE TABLE owed_payment (
  payment_id       INTEGER PRIMARY KEY,
  book_id          INTEGER NOT NULL REFERENCES book (book_id),
  unpaid_id        INTEGER NOT NULL REFERENCES unpaid (unpaid_id),
  amount           INTEGER NOT NULL CHECK (amount > 0),
  paid_on          TEXT NOT NULL,
  money_account_id INTEGER NOT NULL REFERENCES account (account_id),
  entry_id         INTEGER NOT NULL UNIQUE REFERENCES entry (entry_id),
  created_at       TEXT NOT NULL
);
CREATE INDEX owed_payment_by_doc ON owed_payment (unpaid_id);

-- Documents marked paid before 0019 settled in full with one entry: the
-- money line is the debit for collect and the credit for pay.
INSERT INTO owed_payment (book_id, unpaid_id, amount, paid_on, money_account_id, entry_id, created_at)
SELECT u.book_id, u.unpaid_id, u.amount, u.paid_on,
       (SELECT l.account_id FROM line l
         WHERE l.entry_id = u.paid_entry_id
           AND ((u.side = 'collect' AND l.debit > 0) OR (u.side = 'pay' AND l.credit > 0))
         ORDER BY l.line_no LIMIT 1),
       u.paid_entry_id, u.created_at
  FROM unpaid u
 WHERE u.status = 'paid'
   AND u.paid_entry_id IS NOT NULL
   AND u.paid_on IS NOT NULL
   AND NOT EXISTS (SELECT 1 FROM owed_payment p WHERE p.entry_id = u.paid_entry_id)
   AND EXISTS (SELECT 1 FROM line l
                WHERE l.entry_id = u.paid_entry_id
                  AND ((u.side = 'collect' AND l.debit > 0) OR (u.side = 'pay' AND l.credit > 0)));
