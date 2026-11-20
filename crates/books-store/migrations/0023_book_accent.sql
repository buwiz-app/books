-- The Books window's accent colour, one of the app's named presets.
-- A display choice only: nothing in the ledger reads it.
ALTER TABLE book ADD COLUMN accent TEXT NOT NULL DEFAULT 'green';
