# Ferry App — Super-Detailed Build Plan (Step by Step)

Every step to build, wire, test, and ship the Ferry desktop app, in execution
order. Each step names the exact files, commands, events, and the check that
proves it works. Steps are numbered continuously; do them in order — later
steps assume earlier ones are green.

Conventions: `invoke("cmd", {args})` = Tauri IPC from React. Events are
`backup:progress`, `download:progress`, `restore:progress`,
`bootloader:progress`, `cloud:progress`, all shaped
`{stage, current, total, current_item}` (cloud uses
`{uploaded, total, part, parts}`).

---

## Phase 0 — Workstation & repo (Steps 1–6)

- [x] **Step 1.** Install Rust MSVC toolchain + VS Build Tools (C++ workload);
  verify `cargo --version`. The linker (`link.exe`) is mandatory.
- [x] **Step 2.** Install Node 20+; verify `node --version`, `npm --version`.
- [x] **Step 3.** Turn Smart App Control OFF (Windows Security → App & browser
  control) OR sign every dev binary — unsigned `build-script-build.exe` files
  are blocked otherwise (os error 4551).
- [x] **Step 4.** Clone repo, `cd app`, run `npm install`.
- [x] **Step 5.** Confirm `.gitignore` covers `node_modules/`, `target/`,
  `dist/`, `.env` (never commit `assist-server/.env` — it holds a B2 key).
- [x] **Step 6.** Run `npm run tauri dev` once; expect the Welcome screen and
  exactly one Ferry window (no devtools popup unless `FERRY_DEVTOOLS=1`).

## Phase 1 — Backend foundation (Steps 7–14)

- [x] **Step 7.** `src-tauri/src/types.rs`: define `DriveInfo {drive_letter,
  model, total_bytes, free_bytes, is_removable, disk_number, partition_count}`,
  `Manifest/ManifestEntry/SkippedEntry`, `AppEntry`, `DriverEntry`,
  `RestoreSummary`, `UsbLayout {boot_letter, data_letter}`. All serde.
- [x] **Step 8.** `src-tauri/src/safety.rs`: `validate_drive_letter` (single
  A–Z only), `ensure_removable_drive_letter` (cfg-windows `GetDriveTypeW`),
  `drive_root` (always `E:\` spelling), `validate_usb_root`,
  `sanitize_relative_path` (no drive/root/./..), `sanitize_filename`,
  `is_http_url`, `validate_staging_dir` (temp-only), `validate_backup_dir`
  (temp-or-removable; Linux skips the removable check by documented design).
- [x] **Step 9.** Unit-test every helper in `safety.rs` (allowlist, traversal
  `..`, `C:`, absolute paths, overlong filenames). Gate: `cargo test`.
- [x] **Step 10.** `src-tauri/src/hashing.rs`: `hash_file` (SHA-256, 1 MB
  chunks) shared by backup + restore. Test determinism + 64-hex length.
- [x] **Step 11.** `src-tauri/src/lib.rs`: split modules into portable
  (`crypto`, `hashing`, `restore`, `safety`, `types`) vs `#[cfg(windows)]`
  (everything touching diskpart/registry/netsh/winget). Rule: shared code
  must never import OS-only crates.
- [x] **Step 12.** `tauri.conf.json`: window 900×640 (min 640×480),
  restrictive CSP (never `null`), shell plugin `open` only, no
  `shell:allow-execute` in `capabilities/default.json`.
- [x] **Step 13.** `build.rs` + `windows-app-manifest.xml`: embed
  `requireAdministrator`; gate `tauri_build` on Windows target only so the
  Linux CLI cross-builds. Set `[[bin]] test = false` (bin harness inherits
  the manifest and dies with os error 740 unelevated).
- [x] **Step 14.** Gate: `cargo clippy --locked -- -D warnings` clean.

## Phase 2 — Backup engine (Steps 15–26)

- [x] **Step 15.** `backup/paths.rs` `migration_profile(target_id)`:
  detect source OS, map target family (`windows-*`→windows, `ubuntu-*`→linux),
  measure each top-level profile folder (bytes + file count, single walk,
  no symlinks), mark recommended (Desktop/Documents/Downloads/Pictures/
  Music/Videos), attach per-move warnings (Wi-Fi won't transfer to Linux, etc.).
- [x] **Step 16.** Unit-test the family mapping + measure/recommend +
  warning rules (`paths.rs` tests).
- [x] **Step 17.** `backup/scan.rs` `scan_user_files(extra_excludes, roots)`:
  empty `roots` = whole home; otherwise canonicalize each root and
  `ensure_inside` the canonical home (fail closed). Walk canonical paths only
  so `normalise_path` prefix-stripping agrees (verbatim `\\?\` paths would
  otherwise poison every relative path).
- [x] **Step 18.** Segment-aware excludes (folder patterns match whole
  segments, file patterns match file names — never raw substring).
- [x] **Step 19.** OneDrive placeholder detection by file attribute
  (`RECALL_ON_DATA_ACCESS`/`RECALL_ON_OPEN`): list them, warn with count/size
  in ExcludeReview, never silently copy zero-byte stubs.
- [x] **Step 20.** Unit tests: scoped-subset walk, outside-profile rejection,
  exclude rules, normalise (use `target/ferry-test-*` dirs, NOT system temp —
  default excludes skip `AppData\Local\Temp`).
- [x] **Step 21.** `backup/copy.rs` `copy_files_to_usb(app, files, usb_root)`:
  validate USB root, sanitize each relative, `ensure_source_under_home`,
  `ensure_inside` dest, 1 MB chunk copy, `backup:progress` per file.
- [x] **Step 22.** `backup/checksum.rs` `verify_backup(app, state, files,
  usb_root, skipped)`: per file → `verify_one` (hash both sides; on mismatch
  ONE re-copy + re-check, then fail that file with a named reason); cap the
  failure dump at 10 + count; write `manifest.json`; insert the one-time
  erase token into `AppState::verified_roots`.
- [x] **Step 23.** `list_usb_backup_files(usb_root)`: index everything under
  `Backup/` (browser/Wi-Fi/inventory outputs) so aux files JOIN the manifest —
  otherwise they'd be encrypted but never verified or restored.
- [x] **Step 24.** `read_manifest(backup_dir)`: parse + return manifest (feeds
  the edition warning + restore UI).
- [x] **Step 25.** Gate: `cargo test` — scoped walk, rejection, hashing,
  manifest round-trip all green.
- [ ] **Step 26.** Manual: scan a real profile, confirm AppData never walks
  unless ticked, placeholder warning appears if OneDrive is present.

## Phase 3 — Encryption (Steps 27–33)

- [x] **Step 27.** `crypto/keygen.rs`: explicit OWASP Argon2id params
  (m=19456, t=2, p=1); random `SaltString`; DECODE base64 to raw bytes before
  `hash_password_into` (never feed base64 text as salt). Round-trip + length
  tests. NOTE: salt handling changed once already — old-format containers fail
  loudly via the `FERRYENC1` magic check, never silently.
- [x] **Step 28.** `crypto/stream.rs` container: `[FERRYENC1][u32 chunk][nonce(12)+u32 len+ciphertext]*`, fresh nonce per 1 MB chunk, `MAX_RECORD_LEN`
  sanity cap, legacy-header rejection. Round-trip + wrong-key + legacy tests
  (multi-chunk: use ≥2.5 MB fixture).
- [x] **Step 29.** `crypto/encrypt.rs` `encrypt_backup(usb_root, password)`:
  enforce ≥12 chars; validate USB; ZIP to temp file (streamed, `ZipWriter`
  over `BufWriter` + `sync_all`); chunk-encrypt to `Backup.enc.tmp`; fsync;
  rename; stream-decrypt + manifest-check the finished file; zeroize key;
  ONLY then `remove_dir_all(Backup/)`. Clean stale `.tmp` files first.
- [x] **Step 30.** `crypto/decrypt.rs` `run_decrypt` (pub, shared with CLI):
  empty staging → fresh temp dir; re-derive key; chunk-decrypt to temp ZIP;
  manifest-check; `enclosed_name()` extraction only + `ensure_inside`; delete
  temp ZIP; zeroize key.
- [x] **Step 31.** `zeroize` all key material after use; no passwords in logs.
- [x] **Step 32.** Gate: round-trip test with wrong-password and truncated-
  file cases; confirm plaintext `Backup/` is gone only after verification.
- [ ] **Step 33.** Manual: encrypt a real backup, pull the USB mid-write once
  (expect clean failure, plaintext kept), retry succeeds.

## Phase 4 — USB engine (Steps 34–43)

- [x] **Step 34.** `disk/enumerate.rs` `list_removable_drives`: A–Z loop,
  `GetDriveTypeW` removable-only, free/total via `GetDiskFreeSpaceExW`,
  model via `Get-Disk FriendlyName`, then `collapse_to_physical_disks`
  (multi-partition sticks = ONE row, whole-disk size, roomiest free space;
  unresolvable disks listed, never hidden). Unit-test the collapse with the
  Ventoy-stick fixture.
- [x] **Step 35.** `disk/partition.rs` `prepare_usb(drive_letter, boot_mb)`:
  allowlist letter → removable re-check → resolve disk number (single-letter
  PowerShell, no interpolation) → removable re-check → `clean`/`convert mbr`/
  FAT32 `FERRY_BOOT` + exFAT `FERRY_DATA` via a closed-then-deleted temp
  script → resolve BOTH new letters by label with retry → return `UsbLayout`
  (`E:\` spellings via `drive_root`). Clamp boot size 100 MB–32 GB.
- [x] **Step 36.** `find_diskpart_error`: scan stdout markers (diskpart exits
  0 on script failure); temp script removed in all paths; stderr+stdout in
  the error. Unit-test pass/fail/denied fixtures.
- [x] **Step 37.** Order contract (load-bearing): partition runs FIRST, right
  after drive selection, before anything is written — so the wipe cannot
  destroy Ferry's own backup. Guards that remain: removable-only API check,
  explicit UI confirmation, model/size display. (`risks.md` ordering text was
  written for the old order — kept honest in `completion-plan.md` Phase 1.)
- [x] **Step 38.** `disk/bootloader.rs` `write_bootloader(app, boot_letter,
  data_letter, iso_filename)`: validate both letters removable + sanitize
  filename; require ISO on data partition; `Mount-DiskImage` via `-File`
  script with `-IsoPath` arg (never interpolated); require `EFI` dir;
  pre-flight **FAT32 4 GB per-file check** (fail before copying); copy with
  per-file byte-count verify + `bootloader:progress` events; always dismount;
  delete ISO after.
- [x] **Step 39.** Unit-test `fits_fat32` boundaries (1.69 GB pass, 4.7 GB
  wim fail, exactly-4GiB-1 pass).
- [x] **Step 40.** Gate: `cargo test` (collapse, diskpart fixtures, FAT32).
- [ ] **Step 41.** Manual on SPARE USB ONLY: partition → both labels appear →
  letters resolve → re-running on the prepared stick works.
- [ ] **Step 42.** Boot test (the MVP blocker): prepared Ubuntu USB boots on
  real UEFI hardware/VM. Secure Boot off is the accepted fallback.
- [x] **Step 43.** Record bootloader binary provenance (vendor ISO carries
  its own signed `/EFI/boot/bootx64.efi`; Ferry writes no boot config).

## Phase 4b — Portable Ferry.exe on USB (OPEN, not implemented)

mvp.md requires a self-contained portable exe on the data partition so restore
runs on a fresh OS with nothing installed. No code copies it today (verified:
no `Ferry.exe`/portable copy path anywhere in `src-tauri/src`).

- [ ] **Step 43-A.** After `prepare_usb` succeeds, copy the release portable
  EXE to `<data:>\Ferry.exe` (or `Ferry/` dir). Decide artifact name + path.
- [ ] **Step 43-B.** Prove it launches from FAT32/exFAT USB on a fresh
  Windows install with no other Ferry present and WebView2 inbox.
- [ ] **Step 43-C.** Prove the full restore flow from that launch, offline
  (detect → decrypt → verify → copy) before any network is confirmed.

## Phase 5 — OS download (Steps 44–49)

- [x] **Step 44.** `download/sources.rs`: backend-resolved `source_id` ONLY —
  the renderer never supplies URLs/checksums (vendor-only rule).
- [x] **Step 45.** `download/fetch.rs`: sanitize filename (strip query/
  fragment, charset/length rules); resume via `Range` but restart on
  non-206; `error_for_status`; strict `SHA256SUMS` parse (whitespace split,
  exact filename, 64-hex normalize); delete file on mismatch.
- [x] **Step 46.** `os-sources.json`: Ubuntu entries live (24.04.2/22.04.5);
  Windows entries `url: null` BY DECISION (MCT cut) — UI greys them out.
- [x] **Step 47.** Gate: kill a download mid-way → resume works; corrupt the
  file → deleted + clear error.
- [ ] **Step 48.** Manual: full Ubuntu ISO download verifies first try.
- [ ] **Step 49.** v0.2 track (not now): MCT manifest API spike per
  `completion-plan.md` Phase 2.

## Phase 6 — Inventory, browser, Wi-Fi (Steps 50–60)

- [x] **Step 50.** `inventory/apps.rs`: 3 registry hives, skip empties +
  `SystemComponent`, dedupe, sort. USER apps only: also skip drivers, VS/SDK
  components, add-ins, anti-cheat, hardware-vendor publishers (AMD/NVIDIA/
  Intel/Realtek…) and browser web-app shortcuts. Merge `store.rs`
  (`Get-AppxPackage` CSV, fail-open) minus frameworks, packages Windows lists
  as shipped (`AppxAllUserStore\InboxApplications`+`Applications`), codecs/
  language packs, and Microsoft Developer-signed identity stubs. Measured on a
  real Win11 PC: 71 Store rows → only user-chosen ones.
- [x] **Step 51.** `inventory/picker.rs` tiers: winget exact single-match
  (fail closed on ambiguity) → curated table (`curated-apps.json`,
  exact case-insensitive match, URL-validated entries only) → http(s)
  `URLInfoAbout` → Tier 3. Per-app `backup:progress` heartbeats (winget is
  slow — the UI must never look hung).
- [x] **Step 52.** `install_app(winget_id)`: strict `[A-Za-z0-9._-]` allowlist,
  argv array (never shell), accept-source-agreements flags.
- [x] **Step 53.** `inventory/network.rs` (replaced `drivers.rs`): network
  adapters only (name/provider/version) into `drivers.json` — the one driver
  that matters if the new OS comes up offline. Quote-aware CSV parse.
- [x] **Step 54.** `inventory/linux.rs` + `app/linux-equivalents.json` (~45
  entries): write `Backup/linux-apps.md` grouping same-app vs alternative,
  name-but-never-guess unlisted apps.
- [x] **Step 55.** `inventory/save.rs`: validate-then-write
  `apps.json`/`drivers.json` into `Backup/`; `read_inventory` from staging.
- [x] **Step 56.** `browser/chrome.rs`: ALL profiles (`Default`, `Profile N`,
  Guest); per-file skip reasons (locked DB while running); DPAPI limitation
  documented (passwords need browser sync).
- [x] **Step 57.** `browser/firefox.rs`: every profile dir; `places.sqlite` +
  `logins.json` + `key4.db`; Primary-Password caveat documented.
- [x] **Step 58.** `wifi/export.rs`: `netsh … key=clear`; no-interface → Ok(0)
  skip; locale-independent XML-diff counting; `WiFi-Passwords.txt` sheet
  (SSID/security/key, entity-decoded) inside `Backup/`. Parser lives in
  portable `wifi/profile.rs` (shared with `ferry-restore`). netsh runs with
  `current_dir(WiFi)` + `folder=.` — netsh silently truncates absolute paths
  ≳126 chars, collapsing all profiles into one file while saying "success".
- [x] **Step 59.** `wifi/import.rs`: `.xml`-only, 1 MB cap, per-file counts,
  relative `filename=` from inside the folder (same netsh truncation).
- [ ] **Step 60.** Gate: `cargo test` green; manual spot-check apps.json vs
  Add/Remove Programs; Tier 1 hits for Chrome/VSCode.

## Phase 7 — Restore, both platforms (Steps 61–69)

- [x] **Step 61.** Shared core `restore::copy::restore_to_desktop(backup_dir,
  progress_cb)`: manifest parse → sanitize each `backup_path` → containment
  both sides → hash-before-copy → `create_dir_all` errors propagated (never
  `.ok()`) → copy → hash-after-copy → summary `{restored, skipped, failed}`.
- [x] **Step 62.** Windows `restore::detect.rs`: scan removables for
  `Backup.enc` + `backup.salt`, return first valid location.
- [x] **Step 63.** Windows `restore_files` command: thin Tauri wrapper emitting
  `restore:progress` (cfg(windows)).
- [x] **Step 64.** Desktop dir: `%USERPROFILE%\Desktop` on Windows;
  `xdg-user-dir DESKTOP` → `~/Desktop` → `~` fallback chain on Linux.
- [x] **Step 65.** `src/restore_cli/main.rs` (`ferry-restore`): find backup
  (`--exe-dir`, `/media`, `/mnt`, `/run/media` two levels) → `rpassword`
  prompt (reject empty) → shared decrypt → shared verify-copy with
  one-line progress → printed summary (cap 10 failures) → recreate Wi-Fi via
  `nmcli connection add` (WPA/WPA2-PSK + WPA3-SAE + open; skip existing;
  enterprise = "sign in again"; no nmcli → path to WiFi-Passwords.txt) →
  point at `linux-apps.md`. No elevation.
- [x] **Step 66.** Cross-build setup: portable modules (`crypto` minus
  `encrypt`, `hashing`, `restore::copy`, `safety` cfg-gated, `types`);
  `zip` deflate-only; `reqwest` rustls; `[[bin]] test = false`.
- [x] **Step 67.** Gate: `cargo test`, clippy clean; `cargo check --target
  x86_64-unknown-linux-gnu` (add target once) for the shared modules.
- [ ] **Step 68.** Manual Windows restore on a second machine/VM.
- [ ] **Step 69.** Manual Ubuntu restore with `ferry-restore` from USB
  (never yet done — biggest open verification).

### Phase 7b — "Your computer is back" (added 2026-09-23)

- [x] **Step 69a.** One-click app install: `inventory/linux.rs` writes
  `Backup/linux-install.json` (only plain apt/snap hints; alternatives never
  pre-ticked); `ferry-restore` shows a checklist, then re-runs itself via
  `pkexec` as a root helper that accepts only validated `apt:|snap:` args.
- [x] **Step 69b.** Browser data: Chrome/Edge/Brave bookmarks → Netscape HTML
  import files in `Restored/Browser bookmarks`; Firefox places/logins/key4
  copied into the Ubuntu (snap) Firefox profile (`ubuntu/firefox.rs` picks it).
- [x] **Step 69c.** Look and feel: `inventory/personal.rs` saves wallpaper +
  dark mode (`Backup/Settings/`); `ferry-restore` applies via `gsettings`.
  Language/keyboard/time zone deliberately skipped — the Ubuntu installer asks.
- [x] **Step 69d.** Click-to-restore: zenity windows when started without a
  terminal; `Restore with Ferry.desktop` launcher + `ferry-restore` copied to
  the USB by `copy_restore_tool` (binary bundled as a Tauri resource, built by
  `npm run build:restore` in Docker; CI downloads the restore-cli artifact).
- [ ] **Step 69e.** Verify all four on a real Ubuntu 24.04 install (with Step 69).

## Phase 8 — Cloud Backup (Steps 70–77)

- [x] **Step 70.** `assist-server/` (Node/Express, optional sidecar):
  `POST /api/cloud/upload-key|download-key|delete` mint disposable
  backup-scoped B2 keys (single capability, prefix-scoped, short TTL).
  Sponsor features (Nosana/Daytona/DNSimple) dropped 2026-09-23. No auth
  (documented demo gap — see Step 77).
- [x] **Step 71.** B2 master key lives ONLY in `assist-server/.env` (gitignored
  — verify with `git ls-files`). Desktop app never sees it.
- [x] **Step 72.** Rust `cloud/b2.rs`: fetch minted key from sidecar
  (`FERRY_ASSIST_SERVER_URL`, default localhost:8787) → authorize → small
  (≤5 MB single-shot, whole-file SHA1) vs large (100 MB parts, per-part
  SHA1, finish) upload with `cloud:progress` events; download by backup ID;
  best-effort delete after restore.
- [x] **Step 73.** Frontend `CloudUpload.tsx`: upload/skip, progress bar,
  backup-ID write-down ceremony (screen shown only if server
  configured); `RestoreDetect.tsx` cloud-ID restore path.
- [x] **Step 74.** Privacy preserved: only `Backup.enc`+`backup.salt` leave
  the machine; AES-GCM authenticates regardless of storage.
- [ ] **Step 75.** Gate: upload → byte-identical download → delete-confirmed
  against the live bucket (record the run; re-run after any b2.rs change).
- [x] **Step 76.** Documented caps (do NOT ship past demo without fixing):
  no server auth/rate-limit/payment check, keys not revoked early, no
  persistence. See `assist-server/README.md`.
- [ ] **Step 77.** Decide before any public exposure: API-key check minimum
  on the sidecar, or keep Cloud Backup local-demo-only.

## Phase 9 — Frontend shell + screens (Steps 78–100)

- [x] **Step 78.** Shell `App.tsx`: left sidebar (Migrate/Pricing/FAQ/Account)
  + narrow-screen top bar; wizard step machine with Back/Start-over;
  `AuthProvider` wrapper; `src/types.ts` mirroring Rust serde shapes.
- [x] **Step 79.** `src/api.ts`: typed `invoke` wrappers (camelCase args) +
  `assistFetch` (sidecar-gated, no silent calls when unconfigured).
- [x] **Step 80.** `hooks/useTauriProgress.ts` + `components/ProgressBar.tsx`
  (determinate + indeterminate) + `StageList` + `ErrorBox` (friendly message
  + expandable technical detail, `expanded` for fatal errors).
- [x] **Step 81.** `Welcome.tsx`: two doors (Back up / Restore) + admin +
  license notes.
- [x] **Step 82.** `DriveSelect.tsx`: removable-only rows (model, whole-disk
  size, roomiest free space, partition count), rescan, select-to-continue.
- [x] **Step 83.** `PartitionUsb.tsx`: red erase panel naming the exact
  drive, too-small preflight with numbers, checkbox-gated red button,
  single-stage progress, verbatim diskpart errors.
- [x] **Step 84.** `OSSelect.tsx`: Ubuntu cards with sizes; Windows greyed
  with the honest reason (never hidden).
- [x] **Step 85.** `MigrationPlan.tsx`: source→target header, warning box,
  folder checklist with sizes/counts, Recommended-only/All/Clear, Continue
  disabled on empty selection.
- [x] **Step 86.** `ExcludeReview.tsx`: totals, skipped counts, add/remove
  extra excludes with rescan, placeholder warning if OneDrive stubs found.
- [x] **Step 87.** `PasswordSetup.tsx`: ≥12 chars, confirm match,
  write-it-down checkbox gate.
- [x] **Step 88.** `BackupProgress.tsx`: six stages (copy→browser→Wi-Fi→
  inventory→verify→encrypt); aux steps non-fatal with warnings; elapsed +
  last-update heartbeat; USB-file indexing into the manifest; amber box
  wording depends on running/failed/succeeded — NEVER "completed" before
  encrypt finishes.
- [x] **Step 89.** `CloudUpload.tsx` + `DownloadProgress.tsx` (resume %,
  checksum step) + `BootloaderProgress.tsx` (extract %, failure = warning
  on Done, backup stays safe) + `Done.tsx` (boot steps 1-2-3-4).
- [x] **Step 90.** `RestoreDetect.tsx` (USB auto-detect + cloud-ID path) →
  `RestoreProgress.tsx` (decrypt silence note + clock, verify/copy, Wi-Fi)
  → `AppPicker.tsx` (tier badges, Install buttons, edition banner from
  `read_manifest`, informational driver list).
- [x] **Step 91.** `Pricing.tsx` (free-everything, Ferry-managed cloud
  wording tied to the paid model (never "bring your own"), `Faq.tsx` (answers matching CURRENT
  flow: partition-first order, no account anywhere, real cloud behavior).
- [x] **Step 92.** `Account.tsx` hub: Profile / Plan & billing / History /
  Settings tabs; `auth.tsx` local-only accounts (SHA-256 hashed, no server);
  `Settings.tsx` account-only (rename, password, two-step delete); `MyFiles`
  history with two-step clear.
- [x] **Step 93.** Responsive: scrollable content column, sidebar ↔ topbar
  switch under 768px, usable at 640×480.
- [x] **Step 94.** Gate: `npm run build` (tsc+vite) clean; audit prod clean.

## Phase 10 — Wiring matrix (verify every row, Steps 95–104)

| # | Screen → Command | Args | Must hold |
|---|---|---|---|
| 95 | DriveSelect → `list_removable_drives` | — | Only removables; multi-partition sticks collapse to one row |
| 96 | PartitionUsb → `prepare_usb` | `{driveLetter, bootMb}` | Checkbox gate; returns `UsbLayout`; letters resolved by label |
| 97 | OSSelect → `list_os_sources` | — | Null-URL Windows entries render disabled with reason |
| 98 | MigrationPlan → `migration_profile` | `{targetId}` | Source detected; Linux target shows Wi-Fi warning |
| 99 | ExcludeReview → `scan_user_files` | `{extraExcludes, roots}` | Outside-profile roots rejected, plainly |
| 100 | BackupProgress → copy/verify/encrypt + aux set | various | Aux failures warn; copy/verify/encrypt failures stop; manifest covers aux files |
| 101 | CloudUpload → `upload_backup_b2` | `{usbRoot}` | Progress events; backup ID ceremony; skip path works offline |
| 102 | DownloadProgress → `download_os_image` | `{sourceId, destDir}` | Resume, checksum, delete-on-mismatch |
| 103 | BootloaderProgress → `write_bootloader` | `{bootLetter, dataLetter, isoFilename}` | FAT32 preflight; per-file verify; ISO deleted after |
| 104 | Restore chain → detect/decrypt/restore/import + `read_manifest` | various | Edition banner shows; cloud delete best-effort post-restore |

## Phase 11 — QA & acceptance (Steps 105–110)

- [x] **Step 105.** `cargo test` + clippy gate + `tsc` + audit (automated, CI).
- [ ] **Step 106.** Execute `app/test-plan.md` B/R/S/A/P/T sections on real
  hardware; record results in the file.
- [ ] **Step 107.** Adversarial inputs: `C:\Windows` as scan root, `..`
  relatives, 200-char filenames, non-http URLs, 8-char winget IDs with
  spaces, wrong passwords, pulled USB mid-copy, killed download resume.
- [ ] **Step 108.** Cloud path live-fire: big-file multipart upload,
  interrupted resume, wrong backup ID, delete confirmation via listing.
- [ ] **Step 109.** Re-walk the `mvp.md` ship checklist; flip boxes only
  with evidence; document every remaining `[ ]` with a reason.
- [ ] **Step 110.** Boot test + Ubuntu `ferry-restore` run (the two halves
  nobody has executed yet).

## Phase 12 — Release (Steps 111–116)

- [ ] **Step 111.** First green CI run on runners (Windows + ubuntu
  restore-cli job); fix runner-only failures.
- [ ] **Step 112.** Acquire EV certificate; sign installer + portable EXE.
- [ ] **Step 113.** Tag-triggered release: NSIS installer + portable ZIP +
  published checksums on the site.
- [ ] **Step 114.** `os-sources.json` refresh job (scheduled validation of
  Ubuntu point releases).
- [ ] **Step 115.** AV-whitelisting pack (signed binary + explainer) per
  `risks.md`; SmartScreen reputation watch.
- [ ] **Step 116.** Definition of done = mvp.md success metric performed on
  real hardware by a non-technical user, zero file loss, zero partition
  knowledge required.

---

## Appendix — Non-goals (do not build)

Multiboot, VM boot-test, TPM/Secure-Boot bypass, macOS/Linux backup hosts,
permanent cloud storage, telemetry, sponsored picker placement, live web
search for download links. Anything here needs a written product decision
first — see `company/completion-plan.md` Phase 0.
