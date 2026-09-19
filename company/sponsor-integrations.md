# Sponsor Integrations — Implementation Plan

How Ferry uses the three hackathon sponsor products — **Nosana**, **Daytona**,
and **DNSimple** — plus the managed cloud backup they sit alongside. This is
the plan and the honest status of each piece, not a pitch: what's built, what
still needs credentials or a deploy, and what would have to change before any
of it is safe outside a demo.

Implementation lives in [`../assist-server/`](../assist-server/); setup steps
are in [`../assist-server/README.md`](../assist-server/README.md).

---

## Why a sidecar at all

Ferry's core promise is local-first: back up, wipe, reinstall, restore, all
without depending on anyone's server. None of that changes here. The sidecar
exists for two reasons, and both are about things a desktop binary genuinely
cannot do safely on its own:

1. **Secrets can't live in a distributed binary.** Anything embedded in
   `ferry.exe` — a B2 master key, a sponsor API key — can be pulled out of it
   with `strings` or a decompiler. Every credential that matters therefore
   stays server-side, and the app asks the sidecar for narrowly-scoped,
   short-lived permission instead.
2. **Untrusted code shouldn't run on the user's machine.** The AI assist
   feature can propose shell commands. Ferry's whole reputation rests on not
   running unverified things on a machine mid-migration, so proposals get
   executed somewhere disposable first.

Everything in this document is **optional**. With the sidecar unreachable, the
AI assist and share-link features simply don't appear (they're gated on
`VITE_ASSIST_SERVER_URL`), and the rest of Ferry works exactly as before. The
one exception is Cloud Backup, which is by definition a network feature —
see its section below.

---

## 1. Nosana — decentralized LLM inference

**What it is:** a GPU marketplace where jobs are posted as Docker containers,
matched to a node on-chain via Solana, and served from a per-deployment URL.

**What Ferry uses it for:** the reinstall picker sorts apps into three tiers,
and Tier 3 — "no confident source found" — is a deliberate dead end. Ferry
refuses to guess a download link, because guessing is exactly how people land
on fake-installer sites. That honesty leaves the user stuck, though. Nosana
runs an open model that can at least *explain* the app in plain language and
name a well-known cross-platform equivalent, without Ferry ever pretending to
have found an official source.

Running this on a decentralized network rather than a single vendor's API also
fits the product's posture: no one company sees the list of software on a
user's machine.

**How it's wired** (`assist-server/src/services/nosana.ts`):

- A job is deployed once and its endpoint cached for the process lifetime —
  deploying per request would mean waiting on GPU-node pickup every time.
- Inference is called over the standard OpenAI-compatible `/v1/chat/completions`
  shape the model container exposes.
- The system prompt constrains output to strict JSON and forbids suggesting any
  command that deletes, formats, changes permissions, or fetches from an
  unverified URL. Unparseable output degrades to plain text rather than failing
  the request.

**Status: implemented, not yet run against a live job.** Blocked on funding a
Solana wallet with NOS + SOL and exporting a job definition from the Nosana
dashboard. The exact Docker/port schema is deliberately *not* hand-written —
it's exported from their template UI into
`assist-server/nosana-job-definition.json`, because that schema changes with
their templates and inventing one would be guessing.

**Known risk:** first-request cold start blocks on GPU-node pickup, which can
take minutes. For a live demo, warm it up beforehand.

---

## 2. Daytona — sandboxed verification

**What it is:** programmable, isolated Linux sandboxes, created and destroyed
via SDK.

**What Ferry uses it for:** making the AI suggestion above *safe to act on*.
An LLM proposing `flatpak install ...` is a guess. Ferry's rule is that
nothing unverified touches the user's real machine — so before a suggested
command is ever presented as trustworthy, it's executed inside a throwaway
Daytona sandbox. Only a clean exit earns the "✓ Verified in sandbox" badge;
otherwise it stays labelled as an unverified AI suggestion.

This is the part that makes the AI feature consistent with Ferry's principles
rather than a violation of them. The three-tier trust system stays intact —
this adds a *tested* fourth option below Tier 3, it doesn't loosen Tiers 1–3.

**How it's wired** (`assist-server/src/services/daytona.ts`):

- `sandbox.process.executeCommand()` for real shell execution (`codeRun()` is
  for Python/JS snippets in the toolbox language, not arbitrary shell).
- Success is judged on `exitCode`, and the sandbox is deleted in a `finally`
  block so a failure can't leak a running sandbox.

**Status: implemented, needs only an API key.** Verified against the installed
SDK's actual type definitions rather than documentation examples — the method
names in the published docs (`codeRun`, `sandbox.destroy()`) differ from the
current SDK (`executeCommand`, `daytona.delete(sandbox)`).

**Known limitation, stated plainly:** exit code proves the command *ran*, not
that it installed the right thing. This demonstrates the
never-run-untested-code principle; it is not a substitute for a security
review.

---

## 3. DNSimple — share links for cloud backups

**What it is:** REST API for domain registration, DNS records, and automated
Let's Encrypt certificates.

**What Ferry uses it for:** after a cloud backup completes, the user has a
backup ID — an opaque UUID. That's fine for the app to consume but useless for
a human who wants to check on things, or to send to the relative helping them
migrate. DNSimple provisions a real subdomain on demand
(`backup-x7k2.<domain>`) serving a plain status page, so the user gets an
`https://` link instead of an identifier to copy by hand.

**How it's wired** (`assist-server/src/services/dnsimple.ts`):

- `client.zones.createZoneRecord()` per backup, A or CNAME depending on how
  the server is reachable.
- TLS comes from a **wildcard certificate provisioned once, ahead of time**,
  not per link — a fresh DNS-01 challenge takes minutes, which is unusable in
  a live flow.

**Status: implemented, needs a token, account ID, domain, and the one-time
wildcard cert.** Verified against the `dnsimple` package's real type
definitions.

---

## 4. Ferry Cloud Backup — the managed B2 storage these sit alongside

Not a sponsor product, but it's the reason the sidecar handles credentials at
all, and the share-link feature above depends on it.

Cloud Backup is **free and Ferry-managed**: the user creates no account,
bucket, or key. Ferry's own Backblaze B2 master key lives only in the
sidecar's environment. For each upload or restore, the sidecar asks B2 to mint
a **disposable Application Key** scoped to one bucket, one name prefix
(`<backup_id>/`), one capability (`writeFiles` or `readFiles`), and a short
lifetime (6h upload / 30min restore). The desktop app authorizes with *that*
key and transfers bytes straight to B2 — the sidecar is never in the data
path, only the small "mint me a key" call.

Each backup stores `<backup_id>/Backup.enc` and `<backup_id>/backup.salt`.
Both files are needed: without the salt, the password alone cannot derive the
key. After a successful restore the app asks the sidecar to delete both, so
Ferry isn't accumulating everyone's data.

**Status: implemented and verified end-to-end against the live B2 account** —
upload, download (byte-identical round trip), and delete (confirmed gone via a
direct listing). See [`architecture.md`](architecture.md) for the storage
design and [`business-model.md`](business-model.md) for why Ferry absorbs the
cost.

---

## Before any of this leaves a demo

Tracked honestly rather than discovered later:

| Gap | Impact | Fix |
|---|---|---|
| **No auth on any sidecar endpoint** | Anyone who can reach `/api/cloud/upload-key` can store data at Ferry's expense | An API key check at minimum; real auth if Cloud Backup is public |
| **No rate limiting** | Same abuse vector, unbounded | Per-IP / per-key limits on key minting |
| **Minted B2 keys aren't revoked early** | Keys stay valid until expiry even after the transfer finishes | Call `b2_delete_key` once upload/download completes |
| **Sidecar is a single point of failure for Cloud Backup** | Cloud Backup is unavailable if it's down | Deploy properly; the local-first core is unaffected either way |
| **No persistence** | Share-link status entries vanish on restart | A real store if share links outlive a demo |
| **Sandbox verification checks exit code only** | A command can "succeed" having done the wrong thing | Inspect sandbox state, not just status |

---

## Setup order for a working demo

Each step gates the next, and the Nosana one is the long pole:

1. **Nosana first** — fund the wallet, deploy the model job, export the job
   definition. Cold start is the slowest and most failure-prone piece, so find
   out early whether it works.
2. **DNSimple wildcard cert** — issue it well ahead of time; DNS-01
   propagation is not something to do live.
3. **Daytona** — drop in the API key.
4. **B2** — already configured and verified.
5. Run `assist-server` (`npm run dev`), set `VITE_ASSIST_SERVER_URL` in
   `app/.env` to enable the optional frontend features, and set
   `FERRY_ASSIST_SERVER_URL` for the Rust side if it isn't on the default
   `http://localhost:8787`.
