-- The one-time id of each press a person allowed: a second use is refused by
-- the key, whatever else the call says.
CREATE TABLE press_once (
  once TEXT PRIMARY KEY,
  tool TEXT NOT NULL,
  at   INTEGER NOT NULL
);
