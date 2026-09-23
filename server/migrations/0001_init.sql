-- Identity is the email address itself: sign-in is a code sent to it, so
-- there is no password and no separate users table.

CREATE TABLE login_codes (
  email      TEXT PRIMARY KEY,
  code_hash  TEXT NOT NULL,
  expires    INTEGER NOT NULL,          -- epoch ms
  attempts   INTEGER NOT NULL DEFAULT 0,
  sent_at    INTEGER NOT NULL           -- epoch ms, for the resend cooldown
);

CREATE TABLE sessions (
  token_hash TEXT PRIMARY KEY,          -- SHA-256 of the bearer token; the token itself is never stored
  email      TEXT NOT NULL,
  expires    INTEGER NOT NULL
);

CREATE TABLE backups (
  id         TEXT PRIMARY KEY,          -- UUID, also the B2 name prefix
  email      TEXT NOT NULL,
  tier       TEXT NOT NULL,             -- 50gb | 200gb | 1tb
  status     TEXT NOT NULL DEFAULT 'unpaid', -- unpaid | paid | uploaded | deleted | rejected
  created    INTEGER NOT NULL,
  expires    INTEGER,                   -- set when the upload is confirmed
  reminded   INTEGER NOT NULL DEFAULT 0,
  order_id   TEXT
);

CREATE INDEX backups_by_email ON backups(email);
CREATE INDEX backups_by_status ON backups(status);
