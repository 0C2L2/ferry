# Ferry — Status Report

**Date:** 2026-09-23 (updated) · **Version:** 0.1.0 (dev) · **Stack:** Tauri 2 + Rust + React 18 + Tailwind
**Health:** `cargo test` 67/67 pass · `cargo clippy --locked -- -D warnings` clean · `tsc + vite build` clean · `npm audit --omit=dev` 0 vulns

**2026-09-23 (later) — online.** Ferry's server runs on Cloudflare
(`server/`, https://ferry-server.rashidtagaev01.workers.dev, D1 + hourly cron)
and the website is live at https://ferry.rashidtagaev01.workers.dev
(`site/`: landing, download, FAQ, privacy, terms, `/account`, `/admin`).
Cloud Backup is free (50 GB, one per person, 30 days) and needs no account:
the app hands out a **restore code**. Email sign-in (Resend) is built but
switched off until Ferry has a domain — the admin page turns it on. Verified:
40-step local smoke test + live end-to-end run (code → upload → sign in with
the code → delete).

**2026-09-23 update — "your computer is back".** `ferry-restore` now does
more than files, and it finally ships on the USB:
- **Click-to-restore:** the Windows app copies `ferry-restore` (built in
  Docker by `npm run build:restore`, bundled as a Tauri resource) and a
  `Restore with Ferry.desktop` launcher next to `Backup.enc`. Double-click
  (after "Allow Launching") opens zenity windows; a terminal still works.
- **Apps:** a checklist of apps that install with plain apt/snap
  (`Backup/linux-install.json`); alternatives are offered but never
  pre-ticked. One Ubuntu password prompt (pkexec) runs a root helper that
  accepts only validated package names.
- **Browsers:** Firefox bookmarks, history and saved passwords go straight
  into Ubuntu's Firefox; Chrome/Edge/Brave bookmarks become import files.
- **Wi-Fi:** saved networks are recreated with NetworkManager (netsh path
  truncation bug fixed on the Windows side).
- **Look and feel:** wallpaper and dark mode carried over via `gsettings`.
- Also: app list is user apps only (no Microsoft, drivers, OEM tools); sponsor
  integrations dropped; Cloud Backup upload fixed (raw B2 auth token, ≥2 parts
  rule) and is **paid** per `business-model.md` (checkout not built yet).
- Still unverified on real hardware: the full Ubuntu 24.04 run (APP-PLAN 69/69e).

**2026-09-19 update.** Two structural changes and four bugs found by actually
running the thing on real hardware for the first time:

*Structural:*
- Erase/partition now runs immediately after drive selection
  (`PartitionUsb.tsx`), before backup/download write anything. The old order
  ran `diskpart clean` *after* backup and OS download had already written to
  the drive, which would have destroyed both.
- The bootloader is extracted from the downloaded OS ISO itself (no bundled
  binary) as a final step (`BootloaderProgress.tsx`).
- Cloud Backup is **Ferry-managed** — no user account, bucket, or key.
  Verified end-to-end against the live B2 account (upload, byte-identical
  download, delete). See `assist-server/README.md`.

*Bugs found by running it (all fixed, all with tests):*
1. **No elevation.** Ferry had no manifest requesting admin, so `diskpart`
   was refused and a non-technical user would have had no way to know to
   right-click → Run as administrator. Now embedded via `build.rs`.
2. **32 MB boot partition couldn't be formatted FAT32** (minimum is ~33.5 MB —
   Ventoy hits the same wall and uses FAT16 at that size). Now 100 MB.
3. **`diskpart` failures were invisible** — it exits 0 even when commands in
   the script fail, so a failed format read as success and surfaced later as a
   confusing "could not find FERRY_BOOT". Its output is now scanned and
   reported verbatim.
4. **`UsbLayout` returned bare drive letters** (`"D"`) where every backup
   command canonicalizes them as paths, producing "USB path does not exist: D".
   Both drive-path spellings now go through one `safety::drive_root()` helper.

*Bootloader simplified (2026-09-21):* the GRUB-loopback scheme is gone.
Measuring the real Ubuntu 24.04.2 ISO showed its largest member is 1.69 GB —
under FAT32's 4 GB limit — so the limit the loopback trick existed to dodge
never bound. The boot partition is now sized to the image and holds its
extracted contents, booting via the vendor's own signed bootloader with no
Ferry-written boot config at all. OS selection moved before partitioning so
the partition can be sized to the image. *Still unresolved:* Windows MCT
remains a deliberate cut (Phase 2), and nothing has been boot-tested.

**2026-09-23 — Windows→Ubuntu Phase 1 landed.** Ferry's GUI is Windows-only
(diskpart, registry, netsh, winget) and had **zero** `cfg(windows)` gating, so
after installing Ubuntu the user held a USB nothing could open — the restore
half of the headline journey did not exist. Now:

- The library compiles for Linux. Windows-only modules (`backup`, `browser`,
  `cloud`, `disk`, `download`, `inventory`, `wifi`, plus `crypto::encrypt` and
  `restore::detect`) are gated; `crypto`, `restore::copy`, `safety`, `types`
  and the new `hashing` module are shared.
- **`ferry-restore`** (`src/restore_cli/main.rs`) is a small Linux CLI that finds the
  backup, decrypts it, verifies every checksum and copies files to the desktop.
  It *reuses* the crypto and verify-copy code rather than reimplementing it —
  two parsers of one encryption format is how silent corruption ships.
- Verify-then-copy was extracted from the Tauri command into
  `restore::copy::restore_to_desktop`, which takes a progress callback, so both
  platforms run identical restore logic.
- The desktop path is no longer `%USERPROFILE%\Desktop`: Linux asks
  `xdg-user-dir DESKTOP` first, because Ubuntu localises it (`~/Bureau`) and
  hardcoding `Desktop` would silently create a folder the file manager doesn't
  show.
- `zip` lost its default features (bzip2/zstd/lzma) — Ferry only ever writes
  deflate, and dropping them removed a C dependency that blocked cross-building.
- CI gained a `restore-cli` job on `ubuntu-latest` that lints, builds and
  uploads the binary.

**Phase 2 — OneDrive placeholders (data-loss class).** Windows 11 enables Files
On-Demand by default: a placeholder shows its full size in Explorer while its
bytes live in the cloud, so a naive backup reports success having copied
nothing — discovered only after the wipe. `backup/scan.rs` now detects them by
file attribute and `ExcludeReview.tsx` warns with count and size before the
drive is touched, telling the user how to hydrate them. They are still listed,
not silently dropped.

**Phase 3 — Windows→Linux app recommendations.** The reinstall picker's three
tiers are all Windows answers (winget IDs, vendor `.exe` links), useless on
Ubuntu. `app/linux-equivalents.json` (~45 curated entries) plus
`inventory/linux.rs` now write `Backup/linux-apps.md` at backup time, grouped
so *same app* and *alternative* can never be misread as the same claim, with
unlisted apps named rather than guessed at.

Verified: `cargo clippy -D warnings` clean for **both** `x86_64-pc-windows-msvc`
and `x86_64-unknown-linux-gnu`; 39 tests pass; `tsc` clean. **Not** verified:
the CLI has never been run on an actual Ubuntu machine against a real backup,
and placeholder detection has not been tested against a live OneDrive setup.

This report compares the working tree against every spec in the repo, records
where reality diverges, and lists what to do next in priority order.

---

## 1. `app/tech-plan.md` — phased plan vs. reality

| Phase | Spec goal | Status |
|---|---|---|
| 1. Core engine (backup/checksum/encrypt) | CLI-testable backup, verify gate, Argon2 + AES-GCM | **Done, hardened.** Streaming chunked container (`FERRYENC1`), verify-before-delete, 12-char password floor, key zeroization. |
| 2. Disk ops | Partition USB, bootloader, API-level drive safety | **Implemented, pending boot test.** Partition/format runs before anything is written; the OS image is extracted onto a FAT32 partition sized to it, booting via the vendor's own signed bootloader (`disk/bootloader.rs`) — no bundled binary and no Ferry-written boot config. Not yet verified by an actual reboot. |
| 3. OS download | MCT API + Ubuntu direct, resume, checksum | **Ubuntu-only by decision, not gap.** Direct + resume + strict checksum done. Windows MCT is cut for this build (`company/completion-plan.md` D2) — greyed out honestly in `OSSelect.tsx`, not silently broken. |
| 4. Inventory | apps/drivers/browser/Wi-Fi | **Done (with limits).** Registry + winget tiers + pnputil/driverquery + Chromium (all profiles)/Firefox + netsh export/import. No Store-app (`Get-AppxPackage`) pass. |
| 5. Restore engine | Detect → decrypt → verify → `Restored/` + Wi-Fi + summary | **Done, now cross-platform.** Post-copy re-hash included. Verify-then-copy extracted into `restore::copy::restore_to_desktop` and shared with the Linux `ferry-restore` CLI, which is the only way the Windows→Ubuntu journey completes. Never run on a real Ubuntu machine yet. |
| 6. UI screens | 12 screens + components + hooks | **Done (extended).** All 12 wizard screens plus Pricing, FAQ, Account hub (Profile / Plan / History / Settings), sidebar nav. |
| 7. Integration & QA | 7 failure-case gates | **Partial.** Cases covered by code + `app/test-plan.md`, but no spare-USB end-to-end run has been executed yet. |
| 8. Distribution | EV signing, pipeline, source updates | **Partial.** CI builds and uploads the Linux `ferry-restore` binary (`restore-cli` job). No signing, no Windows release build, and CI has still never gone green on a runner. |

## 2. `company/mvp.md` — ship checklist

### Safety
- [x] Non-removable drives unselectable **at API level** (`disk/partition.rs` re-checks `GetDriveTypeW`)
- [x] Model + capacity shown, never bare letters (`DriveInfo`, DriveSelect)
- [x] Erase runs first, before backup/download write anything, so nothing downstream can be lost to it — the old "verified-backup token" gate is retired since there's nothing to verify at that point (`disk/partition.rs`, `PartitionUsb.tsx`)
- [x] Explicit confirmation before erase (`PartitionUsb.tsx` checkbox)
- [x] Manifest written + verified before unlock (`backup/checksum.rs`)
- [x] Restore re-checks checksums before copying (`restore/copy.rs`, plus post-copy re-hash)

### Backup
- [x] Full `C:\Users\<name>` scan, silent cache excludes with counts
- [x] Reviewable/modifiable exclude list (per-run, in ExcludeReview)
- [x] Skipped files with reasons; `Backup/drivers.json` via driverquery + pnputil
- [x] Store apps covered via `Get-AppxPackage` pass (`inventory/store.rs`), merged and deduplicated

### Browser data
- [x] Chrome/Edge/Brave Bookmarks + Login Data, all profiles (`Default`, `Profile N`, Guest)
- [x] Firefox `places.sqlite` + `logins.json` + `key4.db`
- [x] Restored under `Restored\BrowserData\`, never injected
- [~] **Limitation, documented in UI-adjacent code:** Chromium `Login Data` is DPAPI-bound and will not decrypt after reinstall; users need browser sync

### Wi-Fi
- [x] `netsh` export with `key=clear` into `Backup/WiFi/` (locale-independent counting)
- [x] Reimport at restore with success/failure counts in summary

### Encryption
- [x] Password at backup start + confirmation + write-it-down gate (min 12)
- [x] `Backup/` → `Backup.enc` (AES-256-GCM, Argon2id key), plaintext deleted **only after decrypt-verify**
- [x] Manifest inside the encrypted container; Ferry.exe/ISO outside it
- [x] Password prompt before any decrypt; wrong password errors cleanly

### Downloads
- [x] Vendor-direct only (backend-resolved `source_id`; renderer cannot inject URLs)
- [x] Checksum verify, delete + notify on mismatch, Range resume with 206 handling
- [ ] Windows MCT path — **cut, fails closed with a clear error**

### USB self-hosting
- [ ] Ferry portable `.exe` copied to USB during preparation — **not implemented**
- [ ] Portable exe runs offline from USB — **not implemented** (needs Phase 8)

### App & driver picker
- [x] Tier 1 winget lookup (exact, single-match, fail-closed) + working Install buttons
- [x] Tier 2 `URLInfoAbout` restricted to http(s), labelled unverified
- [x] Tier 3 grey, no link/button; no live web search anywhere
- [x] Driver list separate, informational

### UX
- [x] Progress for copy/verify/encrypt/download/write/decrypt/restore
- [x] Plain-language errors with collapsible technical details (no raw dumps)
- [x] Backup + restore summaries with counts and failures

### Legal / distribution
- [ ] Code-signed builds — **no**
- [x] No ISO caching/redistribution (fresh vendor download per request)
- [x] License-reactivation note (Welcome page)
- [~] Edition-matching warning — **done in UI**: `read_manifest` feeds an AppPicker banner showing the backed-up edition

## 3. `company/architecture.md`
- [x] Two-partition layout (FAT32 boot sized to the OS image + exFAT data) implemented in `disk/partition.rs`
- [x] `Restored/` rebuild philosophy implemented
- [x] Path normalization matches code: drive prefix stripped (`Users/...`), full path kept in `manifest.json:original_path` (spec text corrected to match)
- [ ] Cloud overflow/Extra Careful as paid SKUs — not the direction. Implemented instead as free Ferry-managed Cloud Backup (`cloud/b2.rs` + `assist-server`); see `company/business-model.md`.

## 4. `company/app-reinstall-picker.md`
- [x] Three tiers, winget-first, Tier-3-as-feature, curated-table-only (no live search)
- [x] Tier-2 curated table mechanism (`app/curated-apps.json`, exact-match, URL-validated) seeded with human-verified vendor homepages (Steam, Zoom, Slack); still labelled unverified in UI
- [x] Store apps via `Get-AppxPackage` merged into inventory
- [x] Cross-OS equivalent suggestions — `inventory/linux.rs` + `app/linux-equivalents.json` write `Backup/linux-apps.md` at backup time (same-vs-alternative grouped, unlisted apps named not guessed)

## 5. `company/risks.md`
- [x] Wrong-disk guardrails (partition-first order so the wipe can't destroy Ferry's own backup, model display, explicit confirm, removable-only API)
- [~] AV false positives — anticipated in docs; **no code-signing or vendor-whitelisting work started**
- [x] No live-search links anywhere
- [x] No ISO redistribution
- [ ] TPM/Secure Boot bypass correctly **absent** (post-MVP decision, do not add by default)

## 6. `company/business-model.md`
- [x] Core free, no login wall (auth is optional and local-only until server accounts launch)
- [x] Cloud Backup is the one paid feature: Individual one-time per migration by size (~$5/50 GB, ~$15/200 GB, ~$40/1 TB), Corporate per-seat subscription (pricing TBD); B2 backend, users register/sign in
- [x] The two trust rules upheld: no data monetization surface, picker has no sponsored path
- [x] B2 upload/download/delete flow exists (`cloud/b2.rs` + `assist-server`); checkout/entitlements arrive with the accounts launch — uploads run open until then

## 7. README design principles
- [x] Never guess (manifest-resolved sources, exact winget match)
- [x] Honest failures (bootloader/MCT/DPAPI gaps surface as clear messages)
- [x] No binary/registry restore (checklist + install buttons only)
- [x] Wipe is the guarded point of no return (runs first, before any data exists to lose; removable re-check + explicit confirm)

## 8. Open tech debt (non-blocking)
1. `src-tauri/target/` (8.6 GB) and `node_modules/` (129 MB) are local-only but gitignored — fine.
2. `get_drive_model` shells to PowerShell per drive — slow with many drives; cache or batch.
3. No automated UI tests; coverage is backend unit tests + manual `app/test-plan.md`.
4. DevTools gate relies on `FERRY_DEVTOOLS`; release builds exclude it via `debug_assertions`.
5. CI (`.github/workflows/ci.yml`: cargo test + clippy gate, npm build + audit) added but **never run on a runner yet** — needs a first green runshot.

## 9. What to do next (priority order)
1. **Boot-test the USB** on a spare machine or a UEFI VM (QEMU/VirtualBox + OVMF). Still the single biggest unverified risk: no USB Ferry produced has ever been booted. *Needs hardware or a VM.*
1b. **Run `ferry-restore` on a real Ubuntu machine** against a real backup. It compiles and lints clean for Linux but has never executed — the half of the journey it owns is unproven. *Needs an Ubuntu box or VM.*
2. **Finish the real end-to-end run** (test-plan §3 B/R/S cases). The run so far reached the backup stage; copy/verify/encrypt, download, and the new extraction step have not been exercised on real hardware. *Needs you + hardware.*
3. **Portable Ferry.exe onto the USB** + offline-first restore (USB self-hosting checklist). *Needs a USB to verify.*
4. **Phase 8**: EV code-signing, release pipeline beyond CI, `os-sources.json` update job. *Needs accounts, certificates, and budget.*
5. Windows MCT download path — tracked as v0.2, not started (deliberately cut for this build).
6. Sponsor integrations (Nosana / Daytona / DNSimple) — dropped 2026-09-23; they added keys and failure points without serving the core migration.
7. Cross-OS app-equivalent suggestions (picker nice-to-have, software-only, low priority).

## 10. How to verify this report
- `cargo test` in `app/src-tauri` → 53/53
- `cargo clippy --locked -- -D warnings` → clean
- `npm run build` in `app` → clean
- `npm audit --omit=dev` → 0 vulnerabilities
- `npm run tauri dev` → single window, wizard + Pricing + FAQ + Account hub
- Manual cases: `app/test-plan.md`
