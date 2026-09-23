# Ferry — Master Plan (0 → 100%)

The complete plan to design, build, ship, and operate Ferry: the desktop app,
the website that manages users, the cloud backend, distribution, money, and
support. Each part states what gets built, in what order, and what "done"
means. No dates — milestones are dependency-ordered.

---

## 0. Product definition (the anchor everything hangs off)

- **What:** a free Windows desktop app that backs up personal files to a USB
  drive, puts an official OS installer on the same drive, and restores
  everything after reinstall — for non-technical users facing Windows 10 EOL.
- **Who:** home users migrating/reinstalling (primary); repair shops and IT
  departments (second product, later).
- **Promise:** start-to-finish without losing a file, without understanding
  partitions. Backup → verify → erase. Never guess sources. Never restore
  binaries. Core free forever; the only paid feature is Cloud Backup.
- **Success metric:** a non-technical user completes the full loop on real
  hardware (§10 M5).

---

## 1. System overview — three pillars

```
┌──────────────────┐        downloads/auth         ┌──────────────────┐
│   DESKTOP APP    │◄──────────────────────────────│   WEB PLATFORM   │
│  (Tauri, offline │   cloud backup up/down        │ site + accounts  │
│   -first engine) │ ────────────────────────────► │ + billing/admin  │
└────────┬─────────┘                               └────────┬─────────┘
         │ USB / local disk                                │ API keys
         ▼                                                 ▼
   user's PC + USB                              ┌──────────────────┐
                                                │  CLOUD BACKEND   │
                                                │ B2 / GCS / billing│
                                                └──────────────────┘
```

1. **Desktop App** — the product. Works fully offline except OS download and
   (later) cloud up/download. Never requires an account for core flows.
2. **Web Platform** — marketing site, user accounts, dashboard, billing,
   admin console, plus machine APIs the app calls (auth, update manifest,
   cloud credential brokerage).
3. **Cloud Backend** — encrypted blob storage + billing records. Zero-knowledge:
   Ferry servers never see plaintext or passwords.

Build order: App engine first (§3) → release v1.0 (§7) → Web Platform
accounts/billing (§4) → Cloud Backend launch (§5). The app must never hard
depend on the web platform for core flows.

---

## 2. Desktop app build (0 → 100%)

### 2.1 Foundation
1. Tauri + React + Tailwind scaffold, pinned versions, lockfiles committed.
2. IPC conventions: camelCase args in, snake_case structs out; all destructive
   input validated backend-side (drive allowlists, path containment).
3. Capability model: least-privilege permissions; CSP enforced; no shell
   execute from the renderer; devtools gated.
4. Logging + user-facing error contract (plain language + expandable detail).
5. Done = `cargo test`, clippy gate, `tsc+vite` build, audit clean, CI green.

### 2.2 Backup engine
1. Profile-folder measurement + OS-aware migration matrix (source → target).
2. Scoped folder walk (selected roots only), cache/temp excludes.
3. Chunked copy with live progress events.
4. SHA-256 manifest; source-vs-backup verify with one self-healing re-copy.
5. Browser data (all Chromium profiles + Firefox), Wi-Fi export, app/driver
   inventory (registry + Store + winget tiers + curated table) — all
   non-fatal auxiliaries with skip reasons.
6. Done = manifest verified on real hardware; wrong-password-style negative
   cases covered by `app/test-plan.md`.

### 2.3 Encryption
1. Argon2id password → AES-256-GCM chunked stream (`FERRYENC1` container).
2. Atomic write (temp + fsync + rename), decrypt-verify before plaintext
   deletion, key zeroization, 12-char password floor with write-down gate.
3. Done = `Backup.enc` opens only with the password; corruption detected.

### 2.4 USB engine
1. Removable-only enumeration with model/size display.
2. Erase gated by explicit typed confirmation of the physical disk (the old
   verify-token was retired: partitioning now happens FIRST, before backup).
3. `diskpart` partition (FAT32 boot + exFAT data) + explicit user confirmation.
4. Bootloader = the ISO's own signed shim/GRUB, extracted onto the FAT32
   partition Rufus-style (no bundled GRUB binary; Secure Boot stays on).
5. Copy portable Ferry EXE + ISO onto the data partition.
6. Done = USB boots on real test hardware (not the build machine).

### 2.5 OS acquisition
1. Ubuntu direct download + resume + SHA256SUMS verification (built).
2. Windows: time-boxed MCT-endpoint spike → implement, or adopt the fallback
   (Ubuntu-first v0.1; Ferry opens official Microsoft pages and validates a
   user-provided ISO by published hash).
3. `os-sources.json` served as versioned data, updatable without rebuilds.
4. Done = verified ISOs for every offered OS, or fallback formally adopted.

### 2.6 Restore engine
1. Auto-detect `Backup.enc` on removable drives; password prompt.
2. Stream-decrypt to temp staging; Zip-Slip-safe extraction.
3. Manifest verify → copy to `Restored/` → post-copy re-hash; Wi-Fi reimport;
   merged summary; app/driver picker with winget installs.
4. Backed-up OS-edition warning surfaced before reinstall choices.
5. Done = full restore on a second machine/VM per test plan.

### 2.7 UI (all of it, in user order)
Welcome → DriveSelect → OSSelect → MigrationPlan → ExcludeReview →
PasswordSetup → BackupProgress → (CloudUpload) → DownloadProgress →
BootloaderProgress → Done, with PartitionUsb right after OSSelect,
plus RestoreDetect → RestoreProgress → AppPicker, and Pricing → FAQ →
Account hub (Profile / Plan / History / Settings). Sidebar nav, responsive
down to 640×480, plain-language errors, live elapsed/heartbeat progress.
Done = every screen reachable and exercised in `app/test-plan.md`.

### 2.8 Hardening + release engineering
1. Threat review per feature (see `STATUS.md` history for the method).
2. EV code signing for installer, portable EXE, embedded bootloader.
3. Tauri auto-updater with signed update feed.
4. NSIS installer + portable ZIP artifacts from CI; release checklist + tag.
5. Done = SmartScreen-clean install on a fresh machine; update path tested.

---

## 3. Web platform build (0 → 100%) — YES, there is a website that manages users

The website is a separate deployable with four areas. Static marketing can
ship early; accounts/billing arrive with Cloud Backup.

### 3.1 Public site (ships with v1.0)
1. Landing (promise, how-it-works, download button with OS/arch detection).
2. Pricing (Free vs Cloud tiers vs Corporate — mirrors in-app copy).
3. FAQ + docs (install guides, troubleshooting, AV-whitelisting explainer).
4. Download hosting (signed artifacts + checksums published).
5. Done = a stranger can find, trust, download, and install Ferry.

### 3.2 Accounts + auth (ships with Cloud Backup)
1. Email + password signup/login, sessions, password reset; local-only app
   accounts migrate (link code shown in-app, claimed on web).
2. The desktop app signs in **against this API** (never embeds secrets);
   tokens are short-lived, scoped to backup up/download only.
3. GDPR from day one: export + delete-my-data flows.
4. Done = sign up on web → sign in inside the app → token works → delete
   removes everything.

### 3.3 User dashboard
1. My migrations: devices, backup dates/sizes, cloud copies with expiry
   countdowns, restore instructions per migration.
2. Plan management: current tier, upgrade, receipts, cancel.
3. Done = a user can self-serve everything without emailing support.

### 3.4 Billing
1. Provider: Stripe (cards, one-time + subscriptions, receipts, webhooks).
2. Individual: one-time charge per migration by backup size (`business-model.md`).
3. Corporate: per-seat subscription + repair-shop dashboard (migrations per
   machine, backup status) — the second product.
4. Entitlement service: the single source of truth the app and dashboard
   both query (`can_upload`, quota, expiry).
5. Done = test-mode purchase → entitlement enforced in-app → refund path works.

### 3.5 Admin console (internal)
1. User lookup, plan overrides, refund/cancel, support notes.
2. Storage/cost dashboards per provider; abuse alarms (impossible sizes,
   impossible rates).
3. `os-sources.json` + app-update publishing workflow.
4. Done = support can resolve any billing/storage ticket from one screen.

---

## 4. Cloud backend build (0 → 100%, post-MVP)

1. **Storage:** Backblaze B2 for Individual (free egress ≤3× stored);
   GCS/S3 for Corporate (SLA + audit). Never the reverse (egress math).
2. **Protocol:** client encrypts first (same `FERRYENC1` container), chunked
   resumable upload with per-part hashes; disposable per-backup keys minted by
   `assist-server`; server verifies identity + quota only — never content.
3. **Retention:** migration bridge, not forever-storage: delete after restore
   (best-effort from the app), user-visible backup ID, delete-on-request.
4. **Credential brokerage:** short-lived upload/download keys minted per
   migration; no long-lived cloud keys on the device; master key server-side only.
5. **Cost control:** per-migration margin check (stored GB × months +
   egress) against plan price before enabling new tiers.
6. Done = paid migration uploads, restores, expires, and refunds cleanly.

---

## 5. Data, security, compliance

- Plaintext exists only on the user's PC/USB and in RAM during processing.
- Passwords: Argon2id-derived keys, never transmitted; web passwords hashed
  server-side (argon2/bcrypt), never logged.
- Secrets hygiene: no keys in repos, short-lived tokens, credential-manager
  storage on device where applicable.
- Privacy: no telemetry by default; opt-in crash reports; published data
  charter ("we cannot read your backup" — architecturally true).
- Compliance: GDPR export/delete; license honesty (no ISO redistribution,
  ever); AV-whitelisting program alongside releases.

---

## 6. QA + acceptance (gates every milestone)

1. `app/test-plan.md` manual suites (auth, pricing, wizard, safety, restore).
2. Automated: Rust tests + clippy gate, frontend build, dep audit, CI green.
3. Hardware matrix: spare USB + test PC/VM for every release; boot test for
   every bootloader change; fresh-machine install test for every release.
4. Beta ring: internal → friends/repair-shop pilot → public.
5. Each milestone below lists its own exit criteria; nothing advances on
   "probably works".

---

## 7. Milestones M0 → M8 (dependency order)

- **M0 Foundation:** scaffold, IPC rules, CI, test plan file. *(Done)*
- **M1 Engine:** backup → verify → encrypt → restore loop proven on hardware.
- **M2 Bootable USB:** bootloader + portable EXE + boot test on real hardware.
- **M3 OS acquisition:** Ubuntu verified; Windows direct or adopted fallback.
- **M4 Signed beta:** EV-signed installer, pilot users, full test plan green.
- **M5 v1.0 public:** site live with downloads/docs, release tag, support inbox.
- **M6 Accounts + billing:** web auth, dashboard, Stripe, entitlements; app
  sign-in against the API.
- **M7 Cloud Backup launch:** B2 upload/restore/expire, paid migrations live.
- **M8 Corporate:** seats, shop dashboard, GCS backend, audit reports.

## 8. Risks that can still kill this (and the answer to each)

1. No spare hardware for testing → nothing ships without it; buy one USB + use any old PC/VM. *(Needs you.)*
2. Windows ISO route stalls → Ubuntu-first v0.1 (§delivery-plan §5).
3. Bootloader/Secure Boot trouble → documented Secure Boot-off path.
4. AV false positives → signing + reputation + whitelisting program.
5. Cloud unit economics → B2-only for Individual; margin check per tier.

## 9. Execution notes (solo/small team)

Work strictly in milestone order; each milestone ends in a testable artifact;
never start M(n+1) with M(n)'s exit criteria red. Docs that must stay true:
`company/mvp.md` (scope), `company/delivery-plan.md` (route + fallbacks),
`app/test-plan.md` (proof), `STATUS.md` (position). When reality and a doc
disagree, change the doc the same day.
