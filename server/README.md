# Ferry server (Cloudflare Worker)

Email sign-in, Lemon Squeezy payments and short-lived B2 keys for Cloud
Backup. Runs on Cloudflare's free tier (Workers + D1). It never sees backup
contents: those are encrypted on the user's PC and go straight to Backblaze B2
with a key scoped to one backup, one capability, and a few hours.

Replaces the local Express `assist-server/`, which the app no longer uses.

## Sign-in

- **Restore code** (default): `POST /auth/anonymous` → `{token, restoreCode}`.
  The code (`FERRY-XXXX-XXXX-XXXX-XXXX`, stored only hashed) is the owner's
  identity; `POST /auth/restore-code` signs in with it on another computer.
- **Email** (only when switched on in `/admin` — needs Ferry's own domain in
  Resend): the email-code flow below.

## Admin

`/admin/*` needs `Authorization: Bearer <ADMIN_TOKEN>` (wrangler secret; local
copy in `server/.admin-token`, gitignored). The website's `/admin` page edits
the D1 `settings` table: email on/off, Resend key + From, free on/off,
retention (1–30 days), global active-backup cap, per-network daily limit.

## Flow (email mode)

1. `POST /auth/start {email}` → a 6-digit code is emailed (Resend).
2. `POST /auth/verify {email, code}` → session token (30 days, stored hashed).
3. `POST /api/cloud/checkout {tier}` → Lemon Squeezy checkout URL.
4. Lemon Squeezy calls `POST /webhooks/lemonsqueezy` (HMAC-signed) → backup is **paid**.
5. `POST /api/cloud/backups/:id/upload-key` → write-only B2 key for `<id>/`, 6 h.
6. `POST /api/cloud/backups/:id/uploaded` → size checked against the plan,
   30-day clock starts, the backup ID is emailed.
7. Restore: `…/download-key` → read-only key, 30 min. Then `…/delete`.

An hourly cron sends the 3-days-left reminder, deletes expired backups (and
unfinished uploads), and clears abandoned checkouts.

## Run locally

```bash
npm install
cp .dev.vars.example .dev.vars      # fill in the B2 values
npm run db:migrate:local
npm run dev                         # http://127.0.0.1:8788
npm run smoke                       # 40 end-to-end checks (uses real B2)
```

With `DEV_MODE=1` (honoured on localhost only) sign-in codes come back in the
API response, emails are printed instead of sent, and checkouts are marked
paid without Lemon Squeezy.

## Website

`npm run site:deploy` publishes `../site` (static files, `wrangler.site.toml`)
with the newest installer as `/downloads/Ferry-Setup.exe` →
https://ferry.rashidtagaev01.workers.dev.

## Deploy (once the Cloudflare account exists)

```bash
npx wrangler login
npx wrangler d1 create ferry        # paste the id into wrangler.toml
npm run db:migrate
npx wrangler secret put B2_MASTER_KEY_ID          # …and each other key in .dev.vars.example
npm run deploy                      # → https://ferry-server.<account>.workers.dev
```

Lemon Squeezy webhook URL: `https://<worker>/webhooks/lemonsqueezy`, event
`order_created`, with the same secret as `LEMONSQUEEZY_WEBHOOK_SECRET`.
