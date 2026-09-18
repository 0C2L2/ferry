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
| `rusqlite` | Read browser `Login Data` (Chrome) and `places.sqlite` (Firefox) SQLite files |
| `zip` | ZIP container for encrypted backup archive |
| `winreg` | Windows registry access (app inventory) |
| `indicatif` | Progress tracking (feeds into frontend via Tauri events) |

### Frontend (npm)

| Package | Purpose |
|---|---|
| `react` + `react-dom` | UI framework |
| `typescript` | Type safety |
| `tailwindcss` | Utility-first CSS |
| `@tauri-apps/api` | IPC bridge to Rust backend |
| `lucide-react` | Icons |
| `react-router-dom` | Page/step navigation |

---

## Repository structure

```
ferry/
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs             # Tauri app entry point, command registry
│       ├── disk/
│       │   ├── mod.rs
│       │   ├── enumerate.rs    # List removable drives (model, size, free space)
│       │   ├── partition.rs    # Create FAT32 boot + exFAT data partitions
│       │   └── bootloader.rs   # Write UEFI bootloader to boot partition
│       ├── backup/
│       │   ├── mod.rs
│       │   ├── scan.rs         # Walk C:\Users\<name>\, build file list + exclude list
│       │   ├── copy.rs         # Stream files to USB Backup/ folder
│       │   ├── checksum.rs     # SHA-256 per file, write manifest.json
│       │   └── verify.rs       # Re-read and compare checksums
│       ├── encrypt/
│       │   ├── mod.rs
│       │   ├── keygen.rs       # Argon2 password → AES-256 key
│       │   ├── encrypt.rs      # Encrypt Backup/ → Backup.enc, delete plaintext
│       │   └── decrypt.rs      # Decrypt Backup.enc into memory for restore
│       ├── download/
│       │   ├── mod.rs
│       │   ├── sources.rs      # OS source manifest (versioned JSON, data-driven)
│       │   ├── fetch.rs        # Resumable HTTP download, progress events
│       │   └── verify.rs       # Checksum downloaded ISO, delete on failure
│       ├── inventory/
│       │   ├── mod.rs
│       │   ├── apps.rs         # Registry scan → apps.json
│       │   ├── drivers.rs      # driverquery + pnputil → drivers.json
│       │   └── picker.rs       # Winget catalog lookup, tier resolution
│       ├── browser/
│       │   ├── mod.rs
│       │   ├── chrome.rs       # Locate + copy Bookmarks + Login Data (Chrome/Edge/Brave)
│       │   └── firefox.rs      # Locate + copy places.sqlite + logins.json
│       ├── wifi/
│       │   ├── mod.rs
│       │   ├── export.rs       # netsh wlan export profile
│       │   └── import.rs       # netsh wlan add profile (at restore)
│       └── restore/
│           ├── mod.rs
│           ├── detect.rs       # Find Backup.enc on USB automatically
│           ├── copy.rs         # Decrypt → verify → copy to Restored/ on desktop
│           └── summary.rs      # Build restore summary report
│
├── src/                        # React frontend
│   ├── main.tsx
│   ├── App.tsx                 # Router + step flow
│   ├── pages/
│   │   ├── Welcome.tsx         # First-run onboarding, Windows license note
│   │   ├── DriveSelect.tsx     # USB drive picker (model, size, free space)
│   │   ├── OSSelect.tsx        # OS picker (Win 11, Win 10, Ubuntu)
│   │   ├── ExcludeReview.tsx   # Review/modify the backup exclude list
│   │   ├── PasswordSetup.tsx   # Set encryption password (with confirmation + warning)
│   │   ├── BackupProgress.tsx  # Live progress: scan → copy → verify → encrypt
│   │   ├── DownloadProgress.tsx# Live progress: OS download + checksum
│   │   ├── WriteProgress.tsx   # Live progress: partition + bootloader + write
│   │   ├── Done.tsx            # "Remove USB and boot from it" instructions
│   │   ├── RestoreDetect.tsx   # Auto-detect Backup.enc, password prompt
│   │   ├── RestoreProgress.tsx # Live progress: decrypt → verify → copy → Wi-Fi
│   │   └── AppPicker.tsx       # App + driver reinstall checklist
│   ├── components/
│   │   ├── DriveCard.tsx
│   │   ├── ProgressBar.tsx
│   │   ├── ChecklistRow.tsx    # Tier-aware row (Tier 1 button / Tier 2 link / Tier 3 grey)
│   │   ├── PasswordInput.tsx
│   │   └── SummaryPanel.tsx
│   └── hooks/
│       ├── useTauriProgress.ts # Subscribe to Tauri progress events
│       └── useDrives.ts
│
├── os-sources.json             # Data-driven OS source manifest (version, URL, checksum URL)
├── package.json
└── README.md
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
    "notes": "Downloaded via Microsoft Media Creation Tool API"
  },
  {
    "id": "ubuntu-lts",
    "label": "Ubuntu 24.04 LTS",
    "source_type": "direct_url",
    "url": "https://releases.ubuntu.com/24.04/ubuntu-24.04.1-desktop-amd64.iso",
    "checksum_url": "https://releases.ubuntu.com/24.04/SHA256SUMS",
    "checksum_type": "sha256"
  }
]
```

---

## Coding phases

Each phase produces something testable before moving on. Phases 1–3 are the critical path.

---

### Phase 1 — Core engine (no UI)

**Goal**: file backup, checksum, and encryption working as a CLI. This is the most important thing to get right — it handles the user's data.

| Task | Module | Notes |
|---|---|---|
| Set up Tauri project skeleton | `main.rs` | `cargo tauri init`, add all crates to `Cargo.toml` |
| Drive enumeration | `disk/enumerate.rs` | Use `windows::Win32::System::Ioctl` to list removable drives with model + size |
| Directory walker + exclude list | `backup/scan.rs` | `walkdir` crate; exclude list as a `HashSet<PathBuf>` |
| File copy with progress events | `backup/copy.rs` | Stream in 1 MB chunks; emit `backup:progress` Tauri events |
| SHA-256 per file + manifest write | `backup/checksum.rs` | `sha2` crate; write `manifest.json` |
| Manifest verify (pre-erase gate) | `backup/verify.rs` | Re-hash every file and compare; return list of failures |
| Password → AES key derivation | `encrypt/keygen.rs` | Argon2id, random 32-byte salt stored alongside `Backup.enc` |
| Encrypt Backup/ → Backup.enc | `encrypt/encrypt.rs` | AES-256-GCM; stream ZIP of Backup/ into encrypted output |
| Decrypt Backup.enc in-memory | `encrypt/decrypt.rs` | Stream decrypt → verify tag → hand off to restore module |

**QA gate**: run backup on a test folder, verify `Backup.enc` cannot be read without the password, verify decryption restores exactly the original files.

---

### Phase 2 — Disk operations (requires elevated privileges)

**Goal**: partition a USB and write a bootable drive. Test on a spare USB only.

> [!CAUTION]
> Every function in this phase that writes to a disk must be guarded by the drive-safety checks: non-removable drives must be un-selectable at the API level, not just in the UI.

| Task | Module | Notes |
|---|---|---|
| UAC elevation request at startup | `main.rs` | Tauri's `requestAdminPrivileges` or manifest `requireAdministrator` |
| Partition creation (FAT32 + exFAT) | `disk/partition.rs` | Use `diskpart` scripted via `std::process::Command`, or `windows::Win32::System::Ioctl` directly |
| Format partitions | `disk/partition.rs` | `format /FS:FAT32` and `format /FS:exFAT` via Command |
| Write UEFI bootloader | `disk/bootloader.rs` | Embed a pre-built Ventoy/GRUB EFI binary; copy to FAT32 boot partition |
| Safety lock: verify backup before enabling erase | `disk/partition.rs` | Check `verified: true` flag set by Phase 1 verify step |
| Copy `Ferry.exe` onto USB data partition | `disk/partition.rs` | `std::fs::copy` after format |

**QA gate**: prepared USB boots on a test machine (not the user's machine) and shows a boot menu.

---

### Phase 3 — OS image download

| Task | Module | Notes |
|---|---|---|
| Parse `os-sources.json` | `download/sources.rs` | Deserialize with `serde_json`; validate entries on load |
| Windows ISO via MCT API | `download/fetch.rs` | Reverse-engineer or call the same API endpoint MCT uses; emit download progress events |
| Ubuntu ISO direct download | `download/fetch.rs` | `reqwest` with `Range` header for resume; save to USB data partition |
| Checksum verify against vendor hash | `download/verify.rs` | Fetch Ubuntu `SHA256SUMS`, parse, compare; delete file on mismatch |
| Windows ISO checksum | `download/verify.rs` | Microsoft publishes checksums via the MCT API response |

**QA gate**: downloaded ISO matches published checksum; a re-run after a partial download resumes correctly.

---

### Phase 4 — Inventory (apps, drivers, browser, Wi-Fi)

| Task | Module | Notes |
|---|---|---|
| Registry scan → `apps.json` | `inventory/apps.rs` | `winreg` crate; three registry paths; deduplicate by `DisplayName` |
| Winget catalog lookup | `inventory/picker.rs` | Call `winget search --id <name> --exact --accept-source-agreements` as subprocess; parse output |
| Tier resolution logic | `inventory/picker.rs` | Tier 1 if winget match found; Tier 2 if `URLInfoAbout` non-empty; else Tier 3 |
| `driverquery` + `pnputil` → `drivers.json` | `inventory/drivers.rs` | Run as subprocess; parse CSV output; flag `third_party: true` where OEM INF |
| Chrome/Edge/Brave browser backup | `browser/chrome.rs` | Locate profile dir via `%LOCALAPPDATA%`; copy `Bookmarks` + `Login Data` |
| Firefox browser backup | `browser/firefox.rs` | Locate profile via `%APPDATA%\Mozilla\Firefox\profiles.ini`; copy `places.sqlite` + `logins.json` |
| Wi-Fi profile export | `wifi/export.rs` | `netsh wlan export profile folder=<path> key=clear` as subprocess |
| Wi-Fi profile import | `wifi/import.rs` | Iterate exported XMLs; run `netsh wlan add profile filename=<file>` per profile |

**QA gate**: `apps.json` matches what's actually installed (manual spot-check vs. Add/Remove Programs); Tier 1 lookup succeeds for known apps (Chrome, VSCode, Slack); Wi-Fi export/import round-trip tested on a test machine.

---

### Phase 5 — Restore engine

| Task | Module | Notes |
|---|---|---|
| Auto-detect `Backup.enc` on connected USB | `restore/detect.rs` | Scan all removable drives for `Backup.enc` in root |
| Decrypt + stream to restore staging | `restore/copy.rs` | Decrypt in memory; never write plaintext to new OS disk except the final `Restored/` copy |
| Verify checksums from `manifest.json` | `restore/copy.rs` | Re-hash each file after copy; report failures |
| Copy to `Restored/` on new desktop | `restore/copy.rs` | `std::env::var("USERPROFILE")` to find desktop; rebuild folder structure |
| Wi-Fi reimport | `wifi/import.rs` | Reuse Phase 4 import module |
| Build restore summary | `restore/summary.rs` | Count restored, skipped, failed, Wi-Fi networks added |

---

### Phase 6 — UI (all screens)

Build screens in the order the user experiences them. Each screen is a React component that invokes Tauri commands and subscribes to progress events.

| Screen | Key behaviour |
|---|---|
| `Welcome.tsx` | Windows licence note, "this app needs admin access" explanation |
| `DriveSelect.tsx` | List removable drives only; show model + size; block system drives at API level |
| `OSSelect.tsx` | Card per OS with logo; show required USB space |
| `ExcludeReview.tsx` | Show default exclude list; toggle items; add custom paths |
| `PasswordSetup.tsx` | Password + confirm field; strength indicator; "write this down" warning; must re-type confirmation phrase |
| `BackupProgress.tsx` | Four-stage progress bar: scan → copy → verify → encrypt; live file count and speed |
| `DownloadProgress.tsx` | Download progress with speed + ETA; checksum verify step shown |
| `WriteProgress.tsx` | Partition + write + bootloader steps; estimated time |
| `Done.tsx` | Clear "safe to remove USB" + boot instructions with screenshots |
| `RestoreDetect.tsx` | Auto-detect USB with Backup.enc; password prompt |
| `RestoreProgress.tsx` | Decrypt → verify → copy → Wi-Fi reimport; live count |
| `AppPicker.tsx` | Grouped checklist; tier badges; Install / Get from site / No match states |

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
Phase 1  ──▶  Phase 2  ──▶  Phase 3
(backup/       (disk ops)    (download)
 encrypt)           │              │
    │               └──────┬───────┘
    │                      ▼
    └──────────────▶  Phase 4
                      (inventory)
                           │
                           ▼
                      Phase 5
                      (restore)
                           │
                           ▼
                      Phase 6 (UI)
                           │
                           ▼
                      Phase 7 (QA)
                           │
                           ▼
                      Phase 8 (ship)
```

Phases 1–3 can be built and tested without a UI at all (as Rust unit tests and CLI invocations). Phase 6 can be mocked with fake data while Phases 4–5 are still in progress.

