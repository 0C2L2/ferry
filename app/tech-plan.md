# Ferry — Technical & Coding Plan

This document covers the full technology stack, project structure, module breakdown, and phased coding plan for building the Ferry MVP. It is grounded in [`mvp.md`](../company/mvp.md).

---

## Stack decision

### Backend — Rust (via Tauri)

Rust is the right choice for Ferry's core engine for three reasons that matter specifically to this product:

1. **It ships as a tiny self-contained binary.** Ferry must live on a USB stick. Electron bundles a full Chromium (~150 MB). Tauri with a Rust backend produces a portable `.exe` of ~5–10 MB — critical for the "Ferry on the USB" requirement.
2. **Memory safety without a garbage collector.** Raw disk I/O, large file copies, and in-memory decryption are exactly the operations where memory bugs cause data corruption. Rust eliminates whole classes of those bugs at compile time.
3. **First-class Windows API access.** The `windows-rs` crate gives direct access to everything Ferry needs: disk enumeration, registry reads, UAC elevation, `netsh`, `driverquery`, `pnputil`.

### Frontend — TypeScript + React + Tailwind CSS (via Tauri)

- Tauri uses the OS's native WebView (WebView2 on Windows 10/11 — already installed) rather than bundling Chromium.
- React for component-based UI; Tailwind for styling without a large CSS framework.
- All system calls go through Tauri's IPC bridge (`invoke`) — the frontend never touches the OS directly.

### Why not Electron

Electron is ruled out: its binary size (150+ MB) makes the "portable exe on USB" requirement impractical, and the bundled Node runtime isn't needed since Rust handles all system work.

---

## Key dependencies

### Rust crates (backend)

| Crate | Purpose |
|---|---|
| `tauri` | App framework, IPC bridge, WebView2 host |
| `windows` (`windows-rs`) | Windows API: disk, registry, UAC, WMI |
| `tokio` | Async runtime for concurrent file I/O and downloads |
| `reqwest` | HTTP downloads with range-request resume support |
| `sha2` | SHA-256 checksum computation |
| `aes-gcm` | AES-256-GCM authenticated encryption (backup container) |
| `argon2` | Password-based key derivation (user password → AES key) |
| `serde` / `serde_json` | JSON serialization for `manifest.json`, `apps.json`, `drivers.json` |
| `walkdir` | Recursive directory traversal for backup scanning |
| `zip` (deflate only, no C deps) | ZIP layer inside the encrypted container; cross-buildable |
| `winreg` | Windows registry access (app inventory) |
| `zeroize` | Wipe AES keys from memory after use |
| `tempfile` | Atomic temp files (diskpart scripts, staging) |
| `rpassword` | Password prompt for the Linux restore CLI |
| `sha1` + `urlencoding` | B2 upload integrity + filename encoding |

### Frontend (npm)

| Package | Purpose |
|---|---|
| `react` + `react-dom` | UI framework |
| `typescript` | Type safety |
| `tailwindcss` | Utility-first CSS |
| `@tauri-apps/api` | IPC bridge to Rust backend |
| `lucide-react` | Icons |
| `@tauri-apps/plugin-dialog` / `-fs` / `-shell` | Native dialogs, fs, and (open-URL-only) shell access |

---

## Repository structure

```
ferry/
+-- app/
|   +-- src-tauri/                  # Rust backend
|   |   +-- Cargo.toml
|   |   +-- build.rs                # Embeds the requireAdministrator manifest
|   |   +-- tauri.conf.json
|   |   +-- windows-app-manifest.xml# UAC elevation (diskpart needs admin)
|   |   +-- src/
|   |       +-- main.rs             # Thin entry point; calls ferry_lib::run()
|   |       +-- lib.rs              # Tauri builder + command registry
|   |       +-- safety.rs           # Trust boundary: path/letter/filename validation
|   |       +-- types.rs            # Shared serde types (DriveInfo, UsbLayout, Manifest…)
|   |       +-- disk/
|   |       |   +-- enumerate.rs    # List removable drives (model, size, free space)
|   |       |   +-- partition.rs    # FAT32 boot + exFAT data via diskpart
|   |       |   +-- bootloader.rs   # Extract boot files from the downloaded ISO
|   |       +-- backup/
|   |       |   +-- scan.rs         # Walk selected roots, apply excludes
|   |       |   +-- paths.rs        # Migration profile: measure folders, OS matrix
|   |       |   +-- copy.rs         # Chunked copy to USB
|   |       |   +-- checksum.rs     # SHA-256 manifest + verify
|   |       +-- crypto/
|   |       |   +-- keygen.rs       # Argon2id key derivation
|   |       |   +-- stream.rs       # Chunked AES-256-GCM container
|   |       |   +-- encrypt.rs      # Backup/ → Backup.enc (+ backup.salt)
|   |       |   +-- decrypt.rs      # Backup.enc → staging Backup/
|   |       +-- download/
|   |       |   +-- sources.rs      # os-sources.json manifest resolution
|   |       |   +-- fetch.rs        # Resumable download + checksum verify
|   |       +-- inventory/
|   |       |   +-- apps.rs         # Registry uninstall keys
|   |       |   +-- store.rs        # Get-AppxPackage (Store apps)
 |   |       |   +-- network.rs      # Network-adapter preflight (connectivity only)
|   |       |   +-- picker.rs       # Three-tier resolution + winget install
|   |       |   +-- save.rs         # apps.json / drivers.json read+write
|   |       +-- browser/
|   |       |   +-- chrome.rs       # Chromium-family profiles (all profiles)
|   |       |   +-- firefox.rs      # places.sqlite + logins.json + key4.db
|   |       +-- wifi/
|   |       |   +-- export.rs       # netsh wlan export profile
|   |       |   +-- import.rs       # netsh wlan add profile (at restore)
|   |       +-- cloud/
|   |       |   +-- b2.rs           # Managed B2 upload/download/delete
|   |       |   +-- progress.rs     # Cloud progress event payload
|   |       +-- restore/
|   |           +-- detect.rs       # Find Backup.enc on a removable drive
|   |           +-- copy.rs         # Verify → copy to Restored/ + summary
|   |
|   +-- src/                        # React frontend
|   |   +-- main.tsx
|   |   +-- App.tsx                 # Wizard step machine + sidebar nav
|   |   +-- api.ts                  # Typed wrappers: Tauri commands + assist-server
|   |   +-- types.ts                # Shapes mirroring the Rust serde output
|   |   +-- auth.tsx                # Optional local-only profile
|   |   +-- history.ts             # Local backup/restore journal
|   |   +-- pages/
|   |   |   +-- Welcome.tsx         # First-run onboarding, Windows license note
 |   |   |   +-- DriveSelect.tsx     # USB picker: model + whole-disk size, expandable per-partition breakdown
|   |   |   +-- PartitionUsb.tsx    # Erase confirmation + partition (runs FIRST)
 |   |   |   +-- OSSelect.tsx        # OS picker (Ubuntu; Windows greyed out) + .iso dropzone
|   |   |   +-- MigrationPlan.tsx   # Folder selection sized by source→target OS
|   |   |   +-- ExcludeReview.tsx   # Review/modify the backup exclude list
|   |   |   +-- PasswordSetup.tsx   # Encryption password + write-it-down gate
|   |   |   +-- BackupProgress.tsx  # copy → browser → Wi-Fi → inventory → verify → encrypt
|   |   |   +-- CloudUpload.tsx     # Paid Cloud Backup upload + backup-ID ceremony
|   |   |   +-- DownloadProgress.tsx# OS download + checksum
|   |   |   +-- BootloaderProgress.tsx # Install boot files from the ISO
|   |   |   +-- Done.tsx            # "Remove USB and boot from it" instructions
|   |   |   +-- RestoreDetect.tsx   # Detect USB backup, or restore from cloud
|   |   |   +-- RestoreProgress.tsx # decrypt → verify → copy → Wi-Fi
 |   |   |   +-- AppPicker.tsx       # App + network reinstall checklist (+ AI assist) + Wi-Fi viewer
|   |   |   +-- Pricing.tsx         # Free core vs paid Cloud Backup tiers
|   |   |   +-- Faq.tsx
|   |   |   +-- Account.tsx         # Profile / History / Settings tabs
|   |   |   +-- MyFiles.tsx         # Backup/restore history
|   |   |   +-- Settings.tsx
|   |   +-- components/
|   |   |   +-- ProgressBar.tsx     # ProgressBar + StageList
|   |   |   +-- ErrorBox.tsx        # Plain message + collapsible technical detail
|   |   +-- hooks/
|   |       +-- useTauriProgress.ts # Subscribe to Tauri progress events
|   |
|   +-- os-sources.json             # Data-driven OS source manifest
|   +-- tech-plan.md                # This document
|   +-- test-plan.md                # Manual test cases
|   +-- migration-scan-plan.md      # Migration-aware scanning design
|
+-- assist-server/                  # Node server: scoped B2 keys for Cloud Backup
|   +-- src/
|       +-- index.ts                # Express routes
|       +-- services/
|           +-- b2admin.ts          # Mints scoped, disposable B2 keys
|
+-- company/                        # Product & planning docs
+-- README.md
```

---

## Data formats

### `manifest.json` (inside `Backup.enc`)
```json
{
  "ferry_version": "1.0.0",
  "created_at": "2026-09-18T09:00:00Z",
  "source_user": "John",
  "source_os": "Windows 10 Home",
  "files": [
    {
      "original_path": "C:\\Users\\John\\Documents\\taxes\\2023.pdf",
      "backup_path": "Users/John/Documents/taxes/2023.pdf",
      "size_bytes": 204800,
      "sha256": "a3f1..."
    }
  ],
  "skipped": [
    { "path": "C:\\Users\\John\\AppData\\Local\\Temp", "reason": "temp folder" }
  ]
}
```

### `apps.json`
```json
[
  {
    "name": "Visual Studio Code",
    "version": "1.92.0",
    "publisher": "Microsoft Corporation",
    "url_info": "https://code.visualstudio.com",
    "tier": 1,
    "winget_id": "Microsoft.VisualStudioCode"
  }
]
```

### `drivers.json`
```json
[
  {
    "name": "NVIDIA GeForce RTX 4060",
    "inf_name": "nvldi.inf",
    "provider": "NVIDIA",
    "version": "556.12.0.0",
    "third_party": true
  }
]
```

### `os-sources.json` (data-driven, ships with the app, updatable as config)
```json
[
  {
    "id": "windows-11",
    "label": "Windows 11",
    "source_type": "microsoft_mct",
    "url": null,
    "checksum_url": null,
    "approx_bytes": 5800000000
  },
  {
    "id": "ubuntu-2404-lts",
    "label": "Ubuntu 24.04 LTS",
    "source_type": "direct_url",
    "url": "https://releases.ubuntu.com/24.04/ubuntu-24.04.2-desktop-amd64.iso",
    "checksum_url": "https://releases.ubuntu.com/24.04/SHA256SUMS",
    "approx_bytes": 6100000000
  }
]
```
Windows entries carry `null` URLs by decision (MCT cut) — the UI greys them
out instead of pretending they download.

---

## Coding phases

Each phase produces something testable before moving on. Phases 1–3 are the critical path.

---

### Phase 1 — Core engine (no UI)

**Goal**: file backup, checksum, and encryption working as a CLI. This is the most important thing to get right — it handles the user's data.

| Task | Module | Notes |
|---|---|---|
| Set up Tauri project skeleton | `main.rs` + `lib.rs` | `cargo tauri init`, crates in `Cargo.toml`, admin manifest in `build.rs` |
| Drive enumeration | `disk/enumerate.rs` | `GetDriveTypeW` for removable drives; collapse partitions to physical disks |
| Directory walker + exclude list | `backup/scan.rs` | `walkdir`; segment-aware excludes; scoped `roots` from the migration plan |
| Migration profile | `backup/paths.rs` | Measure profile folders; source→target OS matrix + warnings |
| File copy with progress events | `backup/copy.rs` | Stream in 1 MB chunks; emit `backup:progress` Tauri events |
| SHA-256 per file + manifest write | `backup/checksum.rs` | `sha2`; write `manifest.json`; one self-healing re-copy on mismatch |
| Password → AES key derivation | `crypto/keygen.rs` | Explicit OWASP Argon2id params; raw decoded salt bytes |
| Encrypt Backup/ → Backup.enc | `crypto/encrypt.rs` + `crypto/stream.rs` | Chunked AES-256-GCM `FERRYENC1` container; verify-before-delete |
| Decrypt Backup.enc | `crypto/decrypt.rs` | Stream decrypt → verify tag → Zip-Slip-safe extract to staging |

**QA gate**: run backup on a test folder, verify `Backup.enc` cannot be read without the password, verify decryption restores exactly the original files.

---

### Phase 2 — Disk operations (requires elevated privileges)

**Goal**: partition a USB and write a bootable drive. Test on a spare USB only.

> [!CAUTION]
> Every function in this phase that writes to a disk must be guarded by the drive-safety checks: non-removable drives must be un-selectable at the API level, not just in the UI.

| Task | Module | Notes |
|---|---|---|
| UAC elevation at startup | `build.rs` + `windows-app-manifest.xml` | Embedded `requireAdministrator` manifest |
| Partition creation (FAT32 + exFAT) | `disk/partition.rs` | Scripted `diskpart`; runs FIRST (before backup writes), removable re-check + explicit UI confirm |
| Resolve new letters by label | `disk/partition.rs` | `assign` doesn't guarantee letters; poll `Get-Volume` by `FERRY_BOOT`/`FERRY_DATA` label |
| diskpart failure detection | `disk/partition.rs` | Scans stdout (diskpart exits 0 on script errors); temp script always removed |
| Extract ISO onto boot partition | `disk/bootloader.rs` | Mount ISO, copy tree with per-file size verify, FAT32 4 GB pre-flight, delete ISO after |
| Portable Ferry.exe onto USB | — | Still open (mvp ship checklist) |

**QA gate**: prepared USB boots on a test machine (not the user's machine) and shows a boot menu.

---

### Phase 3 — OS image download

| Task | Module | Notes |
|---|---|---|
| Parse `os-sources.json` | `download/sources.rs` | Deserialize with `serde_json`; backend-resolved `source_id` only |
| Windows ISO via MCT API | — | Cut for this build (see `company/completion-plan.md` D2) |
| Ubuntu ISO direct download | `download/fetch.rs` | `reqwest` rustls + `Range` resume (206-checked); save to USB data partition |
| User-supplied .iso (drop/browse) | `download/custom.rs` | Inspect-before-accept; chunked copy with progress; labeled unverified, no vendor checksum |
| Checksum verify against vendor hash | `download/fetch.rs` | Fetch `SHA256SUMS`, strict parse, compare, delete file on mismatch |
| Cloud upload/download/delete | `cloud/b2.rs` + `assist-server` | Free Ferry-managed B2 via minted per-backup keys; master key server-side only |

**QA gate**: downloaded ISO matches published checksum; a re-run after a partial download resumes correctly.

---

### Phase 4 — Inventory (apps, drivers, browser, Wi-Fi)

| Task | Module | Notes |
|---|---|---|
| Registry scan → `apps.json` | `inventory/apps.rs` | `winreg`; three registry paths + Store-app merge; user apps only (patches, updates, runtimes filtered by kind, never by vendor); deduplicate by name |
| Store apps | `inventory/store.rs` | `Get-AppxPackage` CSV parse; framework packages filtered |
| Winget catalog lookup | `inventory/picker.rs` | Exact single-match `--name` search; per-app progress events |
| Tier resolution logic | `inventory/picker.rs` | Tier 1 winget; Tier 2 curated table (`curated-apps.json`) or http(s) `URLInfoAbout`; else Tier 3 |
| Winget install | `inventory/picker.rs` | Strict package-ID allowlist, argv (never shell) |
| Linux equivalents | `inventory/linux.rs` | Curated table → `Backup/linux-apps.md` for Ubuntu restores |
| Save/read inventory | `inventory/save.rs` | Validated JSON round-trip on USB / from staging |
| Network-adapter preflight → `drivers.json` | `inventory/network.rs` | `Get-NetAdapter` CSV parse; provider/version; down adapters flagged. Full driverquery inventory deliberately removed. |
| Chrome/Edge/Brave browser backup | `browser/chrome.rs` | All profiles (`Default`, `Profile N`, Guest); per-file skip reasons |
| Firefox browser backup | `browser/firefox.rs` | Every profile dir; `places.sqlite` + `logins.json` + `key4.db` |
| Wi-Fi profile export | `wifi/export.rs` | `netsh` + locale-independent counting + `WiFi-Passwords.txt` sheet |
| Wi-Fi in-app viewer | `wifi/list.rs` | List SSIDs (no secrets) + per-network reveal from the backup |
| Wi-Fi profile import | `wifi/import.rs` | Iterate XMLs; size-capped; success/failure counts |

**QA gate**: `apps.json` matches what's actually installed (manual spot-check vs. Add/Remove Programs); Tier 1 lookup succeeds for known apps (Chrome, VSCode, Slack); Wi-Fi export/import round-trip tested on a test machine.

---

### Phase 5 — Restore engine

| Task | Module | Notes |
|---|---|---|
| Auto-detect `Backup.enc` on connected USB | `restore/detect.rs` (Windows) / `restore_cli/main.rs` mount walk (Linux) | Scan removables / `/media`+`/mnt`+exe dir for `Backup.enc` + `backup.salt` |
| Decrypt + stream to restore staging | `crypto/decrypt.rs` | Chunked decrypt to temp ZIP; Zip-Slip-safe extract |
| Verify checksums from `manifest.json` | `restore/copy.rs` → `restore_to_desktop` | Shared by GUI + Linux CLI via progress callback; pre- and post-copy hash |
| Copy to `Restored/` on new desktop | `restore/copy.rs` | `USERPROFILE\Desktop`; XDG-aware on Linux with home fallback |
| Wi-Fi reimport | `wifi/import.rs` | Reuse Phase 4 import module (Windows only) |
| Linux restore CLI | `restore_cli/main.rs` | `ferry-restore`: find → password prompt → decrypt → verify-copy; shared code only |
| Build restore summary | `restore/copy.rs` | Count restored, skipped, failed, Wi-Fi networks added |

---

### Phase 6 — UI (all screens)

Build screens in the order the user experiences them. Each screen is a React component that invokes Tauri commands and subscribes to progress events.

| Screen | Key behaviour |
|---|---|
| `Welcome.tsx` | Windows licence note, "this app needs admin access" explanation |
| `DriveSelect.tsx` | List removable drives only; show model + whole-disk size; expandable partition dropdown (letter, label, filesystem, sizes); block system drives at API level |
| `PartitionUsb.tsx` | Erase confirmation + partition/format. Runs immediately after drive selection, before anything is written |
| `OSSelect.tsx` | Card per OS with logo; show required USB space; .iso dropzone + browse with inspect-before-accept |
| `MigrationPlan.tsx` | Folder picker sized per folder, recommendations driven by source→target OS |
| `ExcludeReview.tsx` | Show default exclude list; toggle items; add custom paths |
| `PasswordSetup.tsx` | Password + confirm field; strength indicator; "write this down" warning; must re-type confirmation phrase |
| `BackupProgress.tsx` | Six-stage progress: copy → browser → Wi-Fi → inventory → verify → encrypt; live file count and elapsed clock |
| `CloudUpload.tsx` | Paid Cloud Backup upload; returns a backup ID to write down |
| `DownloadProgress.tsx` | Download progress with speed + ETA; checksum verify step shown |
| `BootloaderProgress.tsx` | Install boot files extracted from the downloaded ISO; failure is a warning, not a dead end |
| `Done.tsx` | Clear "safe to remove USB" + boot instructions with screenshots |
| `RestoreDetect.tsx` | Auto-detect USB with Backup.enc, or restore from cloud by backup ID; password prompt |
| `RestoreProgress.tsx` | Decrypt → verify → copy → Wi-Fi reimport; live count |
| `AppPicker.tsx` | Grouped checklist; tier badges; Install / Get from site / No match states; Wi-Fi viewer; network adapters |

---

### Phase 7 — Integration & QA

Full end-to-end run on a real machine with a real USB, testing all failure cases:

- [ ] Backup interrupted mid-copy (power cut simulation) → resume or fail gracefully
- [ ] Wrong password at restore → clear error, no corrupt output
- [ ] USB too small (backup size > available space) → clear error before starting
- [ ] Download checksum failure → file deleted, user notified, retry offered
- [ ] Browser with no saved passwords → no error, just empty `BrowserData/` folder
- [ ] Wi-Fi export on machine with no saved networks → silent skip with count = 0
- [ ] Antivirus interference with disk write → document expected false-positive behaviour

---

### Phase 8 — Distribution

| Task | Notes |
|---|---|
| EV code-signing certificate | Purchase before any public build; required for SmartScreen reputation |
| Sign all build outputs | `.exe` installer + portable `.exe` on USB + any embedded binaries |
| Build pipeline | GitHub Actions: `cargo build --release`, `tauri build`, sign, produce `.exe` installer + portable `.zip` |
| `os-sources.json` update pipeline | Separate CI job to validate source URLs + checksums on a schedule; push updated manifest without a full app release |

---

## Build order summary

```
Phase 1  --▶  Phase 2  --▶  Phase 3
(backup/       (disk ops)    (download)
 encrypt)           |              |
    |               +------+-------┘
    |                      ▼
    +--------------▶  Phase 4
                      (inventory)
                           |
                           ▼
                      Phase 5
                      (restore)
                           |
                           ▼
                      Phase 6 (UI)
                           |
                           ▼
                      Phase 7 (QA)
                           |
                           ▼
                      Phase 8 (ship)
```

Phases 1–3 can be built and tested without a UI at all (as Rust unit tests and CLI invocations). Phase 6 can be mocked with fake data while Phases 4–5 are still in progress.

