# Ferry — Status Report

**Date:** 2026-09-19 (updated) · **Version:** 0.1.0 (dev) · **Stack:** Tauri 2 + Rust + React 18 + Tailwind
**Health:** `cargo test` 22/22 pass · `cargo clippy --locked -- -D warnings` clean · `tsc + vite build` clean · `npm audit --omit=dev` 0 vulns

**2026-09-19 update:** erase/partition now runs immediately after drive
selection (`PartitionUsb.tsx`), before backup/download write anything — the
old order ran `diskpart clean` *after* backup and OS download had already
written to the drive, which would have destroyed both. The bootloader is now
extracted from the downloaded OS ISO itself (no bundled binary) as a final
step (`BootloaderProgress.tsx`); see `company/completion-plan.md` Phase 1 for
what was verified vs. what still needs a real boot test. Windows MCT is a
deliberate cut for this build, not an unimplemented gap (Phase 2, resolved).
Cloud Backup and everything else is free — no subscription tiers anywhere
(`business-model.md`).

This report compares the working tree against every spec in the repo, records
where reality diverges, and lists what to do next in priority order.

---

## 1. `app/tech-plan.md` — phased plan vs. reality

| Phase | Spec goal | Status |
|---|---|---|
| 1. Core engine (backup/checksum/encrypt) | CLI-testable backup, verify gate, Argon2 + AES-GCM | **Done, hardened.** Streaming chunked container (`FERRYENC1`), verify-before-delete, 12-char password floor, key zeroization. |
| 2. Disk ops | Partition USB, bootloader, API-level drive safety | **Implemented, pending boot test.** Partition/format now runs first (before backup/download); bootloader is extracted from the downloaded ISO itself (`disk/bootloader.rs`) rather than a bundled binary. Verified against the real Ubuntu 24.04.2 ISO's actual file layout; not yet verified by an actual reboot (no UEFI VM/hardware in this environment). |
| 3. OS download | MCT API + Ubuntu direct, resume, checksum | **Ubuntu-only by decision, not gap.** Direct + resume + strict checksum done. Windows MCT is cut for this build (`company/completion-plan.md` D2) — greyed out honestly in `OSSelect.tsx`, not silently broken. |
| 4. Inventory | apps/drivers/browser/Wi-Fi | **Done (with limits).** Registry + winget tiers + pnputil/driverquery + Chromium (all profiles)/Firefox + netsh export/import. No Store-app (`Get-AppxPackage`) pass. |
| 5. Restore engine | Detect → decrypt → verify → `Restored/` + Wi-Fi + summary | **Done.** Post-copy re-hash included; summary merged with Wi-Fi counts. |
| 6. UI screens | 12 screens + components + hooks | **Done (extended).** All 12 wizard screens plus Pricing, FAQ, Account hub (Profile / Plan / History / Settings), sidebar nav. |
| 7. Integration & QA | 7 failure-case gates | **Partial.** Cases covered by code + `app/test-plan.md`, but no spare-USB end-to-end run has been executed yet. |
| 8. Distribution | EV signing, pipeline, source updates | **Not started.** No CI, no signing, no release builds. |

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
- [x] Ventoy-shaped layout (32 MB FAT32 boot + exFAT data) implemented in `disk/partition.rs`
- [x] `Restored/` rebuild philosophy implemented
- [x] Path normalization matches code: drive prefix stripped (`Users/...`), full path kept in `manifest.json:original_path` (spec text corrected to match)
- [ ] Cloud overflow/Extra Careful — backend does not exist; UI marks it planned

## 4. `company/app-reinstall-picker.md`
- [x] Three tiers, winget-first, Tier-3-as-feature, curated-table-only (no live search)
- [x] Tier-2 curated table mechanism (`app/curated-apps.json`, exact-match, URL-validated) seeded with human-verified vendor homepages (Steam, Zoom, Slack); still labelled unverified in UI
- [x] Store apps via `Get-AppxPackage` merged into inventory
- [ ] Cross-OS equivalent suggestions — **not built**

## 5. `company/risks.md`
- [x] Wrong-disk guardrails (verify-before-erase, model display, second confirm, removable-only API)
- [~] AV false positives — anticipated in docs; **no code-signing or vendor-whitelisting work started**
- [x] No live-search links anywhere
- [x] No ISO redistribution
- [ ] TPM/Secure Boot bypass correctly **absent** (post-MVP decision, do not add by default)

## 6. `company/business-model.md`
- [x] Core free, no login wall (auth is optional and local-only)
- [x] Everything is free, including Cloud Backup — no tiers, no subscription, no billing anywhere in the app (`Pricing.tsx`, `Account.tsx` updated 2026-09-19 to drop the earlier paid-tier concept)
- [x] The two trust rules upheld: no data monetization surface, picker has no sponsored path
- [x] Cloud Backup product exists (`cloud/b2.rs`, bring-your-own B2 account) — free, no billing, no server auth

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
1. **Boot-test the USB** on a spare machine or a UEFI VM (QEMU/VirtualBox + OVMF). This is now the single biggest remaining risk — the bootloader-from-ISO code is implemented and verified against the real ISO's structure, but no reboot has actually been attempted. *Needs you + hardware or a VM.*
2. **Real end-to-end run** on a spare USB (test-plan §3 B/R/S cases), now that the erase/backup/download ordering is fixed. *Needs you + hardware.*
3. **Portable Ferry.exe onto the USB** + offline-first restore (USB self-hosting checklist). *Needs a USB to verify.*
4. **Phase 8**: EV code-signing, release pipeline beyond CI, `os-sources.json` update job. *Needs accounts, certificates, and budget.*
5. Windows MCT download path — tracked as v0.2, not started (deliberately cut for this build).
6. Cross-OS app-equivalent suggestions (picker nice-to-have, software-only, low priority).

## 10. How to verify this report
- `cargo test` in `app/src-tauri` → 22/22
- `cargo clippy --locked -- -D warnings` → clean
- `npm run build` in `app` → clean
- `npm audit --omit=dev` → 0 vulnerabilities
- `npm run tauri dev` → single window, wizard + Pricing + FAQ + Account hub
- Manual cases: `app/test-plan.md`
