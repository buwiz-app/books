-- Schema 0020 (M10 parity, unpaid-3): a monthly repeat on an unpaid
-- document. Only the original repeats; each month's copy points back to it
-- and names its month, so a month gets at most one copy. Nothing makes a
-- copy by itself: the user adds each one from the app.

ALTER TABLE unpaid ADD COLUMN repeat TEXT CHECK (repeat IS NULL OR repeat = 'monthly');
ALTER TABLE unpaid ADD COLUMN repeat_of INTEGER REFERENCES unpaid (unpaid_id);
ALTER TABLE unpaid ADD COLUMN repeat_month TEXT;
CREATE UNIQUE INDEX unpaid_repeat_once ON unpaid (repeat_of, repeat_month) WHERE repeat_of IS NOT NULL;
