# Ferry — Next Steps (step by step)

**Updated:** 2026-09-23 · Long-range reference: [`MASTER-PLAN.md`](MASTER-PLAN.md)

Each phase depends on the one before it. **You** = needs your hands, an
account, or a decision. **Me** = built in the repo.

**Stack (all free tiers, $0/month):** Cloudflare Pages · Workers · D1 · R2 ·
Backblaze B2 · Resend · Lemon Squeezy · GitHub Actions · SignPath (open source).
**Domain:** free options first — GitHub Student Pack (real domain, free 1 yr)
or `ferry.eu.org` (free forever, slow manual approval); else ~$10/yr on
Cloudflare. Not a blocker: everything runs on `pages.dev`/`workers.dev` and
Resend test mode (emails only you) until the domain exists.

---

## Phase 1 — Prove the real journey — ✅ done 2026-09-23 (worked for v1)

1. **You:** install `Ferry_0.1.0_x64-setup.exe`, back up to the USB with
   Ubuntu 24.04 as the target. Check: no flashing windows, only your own apps
   listed, Wi-Fi exported.
2. **You:** boot a second machine from the USB (Secure Boot on) and install
   Ubuntu 24.04.
3. **You:** on Ubuntu, open the USB → right-click **Restore with Ferry** →
   *Allow Launching* → double-click. Check: files in *Restored*, app checklist
   installs, Firefox data + bookmark files, Wi-Fi reconnects, wallpaper + dark mode.
4. **Me:** fix whatever broke.

**Done when:** Windows → USB → Ubuntu works end to end on real hardware.

## Phase 2 — Accounts setup (you, ~30 minutes, can run during Phase 1)

5. ✅ Domain **ferryapp.download** (Cloudflare DNS). Live: `ferryapp.download` +
   `www.` (website), `api.ferryapp.download` (server). The workers.dev
   addresses stay on for older app builds.
6. ✅ Cloudflare account (connected via `wrangler login`, no token needed).
7. ✅ Resend API key stored — but **email is switched off** (no domain yet). Users
   get a **restore code** instead. Turn email on later from the website's admin
   page (`/admin`): key, From address, test email, on/off switch.
8. ~~Lemon Squeezy store~~ — **deferred.** The account couldn't be opened,
   so **Cloud Backup is free for now** (`CLOUD_FREE = "1"` in
   `server/wrangler.toml`: 50 GB cap, one active backup per email, sign-in
   required). The payment code stays in place; switching payments on later
   means a working provider (Lemon Squeezy, or Polar / Paddle) + one flag.
9. ✅ 30-day retention (maximum) and **GPL-3.0** license confirmed.

## Phase 3 — Ferry server on Cloudflare (me) — ✅ live

Lives in [`server/`](server/README.md), deployed at
`https://ferry-server.rashidtagaev01.workers.dev` (D1 `ferry`, APAC; secrets
set in Cloudflare; hourly cron on). Cloud is free (`CLOUD_FREE = "1"`).

10. ✅ Worker replaces `assist-server` (kept until Phase 4 switches the app).
11. ✅ D1 tables `login_codes`, `sessions`, `backups` (email is the identity —
    no users table).
12. ✅ Email-code sign-in: 10-minute codes, 5 attempts, 60 s resend cooldown,
    30-day sessions; codes and tokens stored only as hashes.
13. ✅ Lemon Squeezy checkout + HMAC-verified `order_created` webhook (store
    and tier must match). Upload keys only for a signed-in owner of a **paid**
    backup — the free-storage hole is closed on this server.
14. ✅ Emails: backup ID + restore steps after upload, 3-days-left reminder,
    deleted notice, oversize rejection.
15. ✅ Hourly cron: reminders, expiry deletes (incl. unfinished uploads),
    abandoned-checkout cleanup. Uploads larger than the plan are deleted.

Verified: 32-step smoke test (`npm run smoke`, real B2) + cron run against
seeded rows + live probes. **Waiting on:** the domain (so sign-in codes reach
anyone, not just the Resend account owner); payments are deferred.

## Phase 4 — App uses the live server (me) — ✅ done

16. ✅ Cloud screen: sign in with an emailed code → upload (free, 50 GB) →
    the backup ID is emailed. Paid checkout is stubbed until payments return.
17. ✅ Restore from cloud: sign in → pick your backup → password.
18. ✅ All server calls go through Rust (session token never reaches the
    webview); release builds point at the Worker URL (`FERRY_SERVER_URL`
    overrides it for local dev). `assist-server/` is no longer used.

## Phase 5 — Website (me) — ✅ live at https://ferryapp.download

Static site in [`site/`](site/), served by a Cloudflare Worker (static assets,
free); deploy with `npm run site:deploy` in `server/` (copies the newest
installer to `/downloads/Ferry-Setup.exe`).

19. ✅ Landing: promise, how it works, what comes back, download (+ SmartScreen note).
20. ✅ FAQ, privacy, terms. **Still missing: a support contact** (privacy page says "coming").
21. ✅ `/account`: sign in with the restore code (or email, when on) → backups,
    delete date, delete now.
22. ✅ `/admin` (token in `server/.admin-token`): stats, email on/off + Resend
    key + From + test email, free on/off, retention (≤30 d), global cap,
    per-network daily limit, list/delete any backup.
    Checkout pages: not needed while cloud is free.

**Done when:** ✅ a stranger can find Ferry, download it, and manage their cloud copy.

## Phase 5a — Accounts & redesign (me) — ✅ live

- Server: rate limits on every sign-in route (D1 `rate_limits`), link a
  restore-code backup to an email account, "delete my account and data",
  admins by email (list edited on `/admin`) as well as the token, branded
  HTML emails with optional Reply-To. 53-step smoke test passes.
- Website redesigned with the installed skills (redesign-skill,
  design-motion-principles): self-hosted Geist, one warm accent, product
  shot hero, timeline steps, bento features, two-column FAQ, custom 404,
  skip link, focus rings, inline confirms, empty/loading/error states,
  scroll reveals that stop under reduced motion; CSP no longer allows inline
  styles. Account page stays signed in (localStorage, 30-day sessions).

## Phase 5b — Email on the domain

E1. ✅ Domain registered in Resend (id `7fbe0cb9…`, us-east-1). Needs 4 DNS
     records: DKIM TXT `resend._domainkey`, MX + SPF TXT on `send`, CNAME `rsend`.
E2. ✅ DNS added via API token (`server/.cf-token`, gitignored): the 4 Resend
     records + DMARC `_dmarc` `v=DMARC1; p=none;`.
E3. ⏳ Resend verification → test email → email on in `/admin`. Already set:
     From `Ferry <hello@ferryapp.download>`, Reply-To `support@…`, admin
     email `admin@…`. Restore codes keep working alongside.
E4. ✅ Email Routing on: `support@` and `admin@` forward to your inbox. Contact
     link on the privacy page and every footer.
E5. Optional, **you:** Cloudflare → ferryapp.download → Web Analytics → off
     (Cloudflare injects a beacon; the site's security policy blocks it anyway).

## Phase 6 — Open source + release v1.0

23. **Me:** add `LICENSE`, `CONTRIBUTING.md`, security policy; final secret
    sweep (already verified: no key was ever committed).
24. **You:** make the GitHub repo public; apply to SignPath Foundation.
25. **Me:** CI on a version tag: build `ferry-restore` (Linux) → build the
    installer → SignPath signs it → upload to R2 + GitHub Releases → the
    website's download button points at it.
26. **Me:** version 1.0.0, README, STATUS.

**Done when:** v1.0 downloads from the website, signed, with no SmartScreen warning.

## Phase 7 — App matching v2 ("find my apps, install what I tick")

Already working: registry + Store scan → 46 curated matches (same /
alternative / none) → `linux-install.json` → tick-list on Ubuntu → one
password → apt/snap install, package names validated before reaching root.
Gaps: coverage (46 apps), you can only choose on Ubuntu, catalog ships inside
the app, unknown apps get a shrug.

**No AI model in the product.** The output becomes a root install command; a
hallucinated or look-alike package name is a security hole, the app list is
private, and per-user LLM calls aren't free-tier. AI is used *offline* to
draft catalog entries, which a human reviews and CI verifies.

A1. ✅ **Catalog v2: 46 → 234 entries.** `aliases` (Store IDs, renames),
    word-boundary longest-name matching, new kind `builtin` ("Nothing to
    install": web apps, antivirus, codec packs), `snap_publisher` pins.
    Fixed on the way: `steam`/`pinta`/`freecad` weren't in 24.04's apt,
    LibreOffice isn't preinstalled on 24.04, VirtualBox never matched.
A2. ✅ **`npm run catalog:verify`** (app/): every apt package exists in
    24.04, every snap exists with the right confinement and a verified /
    starred / pinned publisher. **Me (Phase 6):** run it in CI.
A3. ✅ **Curation workflow:** `node scripts/catalog.mjs find "App"` → draft
    (Claude) → human review of the diff → verify.
A4. **Choose on Windows.** "Your apps" step in the migration plan: installs
    (ticked), alternatives (unticked, with the trade-off), no Linux version,
    not recognised. Choices ride in the backup; Ubuntu asks for one password.
A5. **Signed remote catalog.** Site serves `catalog.json` + an Ed25519
    signature; the app checks it (key built in) and falls back to the bundled
    copy. New apps without a new release; a hacked CDN can't inject packages.
A6. **Opt-in "help Ferry learn".** Unrecognised app names (names only, no
    user or IP) → server counts → admin shows "most wanted" → curate the top.
A7. Later: exact IDs via `winget export`; vendor .debs (Chrome, Edge, Zoom)
    from pinned official URLs; Flathub. Each adds a package source — decide first.

**Done when:** ≥80% of detected apps are recognised, and every install
Ferry offers is verified to exist.

## Later

User dashboard extras, admin console, Linux Mint/Fedora targets, Windows → Windows.
