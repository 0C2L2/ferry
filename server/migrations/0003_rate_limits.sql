-- Fixed-window counters for the sign-in routes (per network, per address).
-- Old windows are cleared by the hourly sweep.
CREATE TABLE rate_limits (
  key          TEXT PRIMARY KEY,   -- e.g. 'start:ip:<sha256>'
  window_start INTEGER NOT NULL,   -- epoch ms
  count        INTEGER NOT NULL
);
