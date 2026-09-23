# Ferry server

A small standalone Node/Express sidecar, separate from the core Tauri/Rust
app on purpose: it keeps every credential that would be dangerous to embed
in a distributed desktop binary (the B2 master key) on a
server Ferry controls, never inside the app itself.

It serves one feature, Ferry Cloud Backup (Rust calls it directly). This is
what "managed cloud storage" means: the desktop app never holds Ferry's real B2 credentials, so
this sidecar (or wherever it's deployed for real use) has to be running for
Cloud Backup to work at all.

## Ferry Cloud Backup (managed — paid per `company/business-model.md`)

The desktop app never touches Ferry's real B2 account. For every backup,
`services/b2admin.ts` asks B2 to mint a **brand-new, disposable Application
Key** — scoped to one bucket, one `namePrefix` (`<backupId>/`), one
capability (`writeFiles` for upload, `readFiles` for restore), and a short
lifetime (6h / 30min). The desktop app then authorizes with *that* key
directly against B2 for the actual file transfer, so this server's bandwidth
is never in the path for the bytes themselves — only the small "mint me a
key" round trip is. After a successful restore, the desktop app asks this
server to delete that backup's files.

### Setup

1. Create a Backblaze B2 account and a bucket (any name).
2. Get the account's **Master Application Key** ID + secret from the B2
   dashboard's App Keys page — not a bucket-restricted key, since minting
   new keys (`b2_create_key`) needs the `writeKeys` capability, which only
   the master key (or an equivalent custom key) has.
3. Set `B2_MASTER_KEY_ID`, `B2_MASTER_APPLICATION_KEY`, `B2_BUCKET_NAME` in
   `.env`. **This key must never go into the desktop app or get committed
   anywhere** — it lives only in this server's environment.

## Setup

```bash
cd assist-server
npm install
cp .env.example .env
```

## Run

```bash
npm run dev
```

Defaults to `http://localhost:8787`. The Rust side finds it via
`FERRY_ASSIST_SERVER_URL` (defaults to `http://localhost:8787` if unset —
fine for local dev, but a real deployment needs this set to wherever the
server actually runs). The frontend shows the cloud screens only when
`app/.env`'s `VITE_ASSIST_SERVER_URL` is set (see `app/.env.example`).

## Endpoints

| Method | Path | Body | Used by |
|---|---|---|---|
| POST | `/api/cloud/upload-key` | — | `cloud/b2.rs` (core, always active) |
| POST | `/api/cloud/download-key` | `{ backupId }` | `cloud/b2.rs` (core, always active) |
| POST | `/api/cloud/delete` | `{ backupId }` | `cloud/b2.rs` (core, always active) |

## Honesty note

This is hackathon-scoped: no persistence, and
**no auth on any endpoint**, including `/api/cloud/upload-key` — anyone who
can reach this server can currently mint an upload key and store data at
Ferry's expense. Fine for a demo behind a local/tunnel URL nobody else knows;
add real auth (even just an API key check) before this is a public endpoint.
Minted keys aren't explicitly revoked early, only left to expire on their own
(6h/30min) — for real usage, `b2_delete_key` after a completed
upload/download would tighten that window further.

