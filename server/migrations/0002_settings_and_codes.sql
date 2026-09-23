-- Admin-editable settings (email on/off, Resend key, limits). Missing keys
-- fall back to defaults in src/settings.ts.
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Sign-in without email: a restore code (shown once, stored only as a hash)
-- stands in for an address. The identity is 'anon:<uuid>' and is used in the
-- `email` columns of sessions/backups exactly like an address would be.
CREATE TABLE restore_codes (
  code_hash TEXT PRIMARY KEY,
  identity  TEXT NOT NULL,
  created   INTEGER NOT NULL
);

-- SHA-256 of the client IP (never the IP itself), for the per-network daily limit.
ALTER TABLE backups ADD COLUMN ip_hash TEXT;
