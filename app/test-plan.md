# Ferry — Test Plan

How to manually test the app end-to-end, plus automated checks. Covers the
wizard, Pricing/FAQ/Auth/Settings/My Files pages, and the safety rules in
`company/risks.md` and `company/mvp.md`.

---

## 1. Environments & prerequisites

- OS: Windows 10/11 (primary). Smart App Control must allow unsigned dev builds,
  or use a signed release build.
- One **spare USB drive** (nothing important on it — tests erase it).
- Optional second PC or VM for restore-side testing.
- Run the app: from `app/`, `npm run tauri dev` (needs Rust MSVC toolchain +
  VS Build Tools with the C++ workload).
- Automated: `cargo test` (in `app/src-tauri`), `npm run build` (tsc + vite),
  `npm audit --omit=dev`.

## 2. Test account & data

- Sign up in-app (Sign in → Create account) with a throwaway email, e.g.
  `tester@example.com` / password `test-password-123`.
- All auth data lives in browser localStorage (`ferry.accounts`, `ferry.session`,
  `ferry.history`) — inspect via DevTools (`FERRY_DEVTOOLS=1`). Sign out to
  start a session clean.

## 3. Functional cases

### Auth & account

| ID | Steps | Expected |
|---|---|---|
| A1 | Sign up with invalid email / short password / mismatched state | Clear inline error, no account created |
| A2 | Sign up valid → sidebar shows first name → reload app | Session persists |
| A3 | Sign out → sign in with wrong password | "Wrong email or password", stays signed out |
| A4 | Sign in correct → sign out | Header returns to "Sign in", session key removed |

### Pricing

| ID | Steps | Expected |
|---|---|---|
| P1 | Open Pricing | Ferry Free $0 card + Cloud Backup tiers (~$5/50 GB, ~$15/200 GB, ~$40/1 TB) + Corporate option; no checkout yet |
| P2 | Click a tier signed out | Redirects to Account sign-in |
| P3 | Sign in, pick 200 GB tier, check Account | "✓ planned" badge; preference shown; unchoose clears it |
| P4 | Choose Corporate | "✓ Corporate planned" + no-commitment note |

### Backup wizard (needs spare USB)

As of 2026-09-19 the erase/partition step runs immediately after drive
selection — before backup, cloud upload, or download ever write anything —
so nothing downstream can be lost to it. The bootloader is installed as a
separate final step, extracted from the downloaded OS image itself (see
`company/completion-plan.md` Phase 1).

| ID | Steps | Expected |
|---|---|---|
| B1 | Welcome → Back up, no USB inserted | "No removable USB drives found" |
| B2 | Insert USB → Rescan → select drive → Continue | Selection highlighted; advances to the OS picker |
| B2b | Multi-partition stick: expand the drive row | Each partition listed with letter, label, filesystem, size, free; selecting still takes the whole disk |
| B3 | OS picker | Ubuntu selectable; MCT (Windows) options marked unavailable in this build |
| B4 | Prepare USB drive, box unchecked | "Erase & prepare drive" stays disabled. Screen states the boot partition size, derived from the chosen image |
| B4b | Select a USB smaller than the image, reach Prepare USB | Blocked up front with "This drive is too small", not a raw diskpart error |
| B5 | Check box → Erase & prepare drive | Partition/format passes; drive shows FERRY_BOOT (FAT32, sized to the image) + FERRY_DATA (exFAT) in Explorer |
| B5b | Migration plan | Source OS detected; folders listed with sizes; recommended pre-ticked; Linux target shows Wi-Fi warning; Continue disabled with nothing ticked |
| B5c | Drop a .txt file / accept an .iso on OSSelect | Non-ISO rejected with plain error; valid ISO shows name + size and feeds partition sizing + copy step |
| B6 | Exclude review | File count + size shown; add/remove extra exclude rescans |
| B7 | Password: short / mismatched / unchecked box | Blocked with plain-language error |
| B8 | Backup progress | All 6 stages complete against the FERRY_DATA partition; manifest noted |
| B9 | Cloud upload screen (needs `assist-server` running with B2 master key configured) | "Back up to Ferry Cloud" uploads with no fields to fill in; success shows a backup ID with a "write this down" prompt; "Skip" advances either way |
| B9b | Restore screen → "Lost your USB? Restore from Ferry Cloud instead" | Backup ID + password download and decrypt via the cloud path; a wrong/unknown ID fails closed with a clear error |
| B9c | Restore from cloud, then complete restore | Backend deletes both cloud files for that backup ID afterward (verify via B2 dashboard or a repeat download-key call failing) |
| B10 | Download Ubuntu | Progress %, checksum verified; corrupt download deleted + error |
| B11 | Finishing USB (extraction step) | Whole ISO unpacked onto FERRY_BOOT with live progress; `/EFI/boot/bootx64.efi` present afterwards; the `.iso` is deleted from FERRY_DATA to reclaim space. A failure here shows a warning on Done but the backup is untouched |
| B12 | My Files after B8 | New "Backup" record with drive, OS, file count, size |

### Safety rules (must all hold)

| ID | Check |
|---|---|
| S1 | System (non-removable) drives never appear in DriveSelect |
| S2 | Prepare-USB re-checks removability at the OS level immediately before `diskpart clean` runs, even if the frontend is tampered with |
| S3 | Explicit erase confirmation checkbox required before "Erase & prepare drive" is clickable |
| S4 | Wrong backup password at restore → clear error, no partial/corrupt output |
| S5 | USB unplugged mid-copy → failure surfaces, no silent success |
| S6 | Extraction step fails closed with a clear error if the OS image has no `EFI` directory |

### Restore & picker (needs prepared USB + password)

| ID | Steps | Expected |
|---|---|---|
| R1 | RestoreDetect with no Ferry USB | "No Ferry backup found" |
| R2 | Correct USB + wrong password | Decrypt fails cleanly |
| R3 | Correct password | Decrypt → verify → copy → Wi-Fi; summary counts; files under `Restored/` on desktop, incl. WiFi-Passwords.txt + BrowserData |
| R3b | AppPicker Wi-Fi section | SSIDs listed without secrets; Show password reveals per network; open networks labeled, not blank |
| R3c | Network adapters section | Adapter names + provider/version shown; down adapters flagged |
| R4 | My Files after R3 | New "Restore" record with counts |
| R5 | AppPicker | Tier badges; Tier 1 Install runs winget; Tier 2 shows vendor-site note; Tier 3 grey, no button |
| R6 | Drivers section | Listed as informational only |

### Settings (Account → Settings tab)

| ID | Steps | Expected |
|---|---|---|
| T1 | Change display name → check sidebar nav | New name shown immediately |
| T2 | Change password with wrong current / mismatched confirm | Blocked with plain-language error; old password still works |
| T3 | Change password correctly → sign out → sign in with new password | Sign-in succeeds |
| T4 | Delete account: cancel at confirm | Account kept, still signed in |
| T5 | Delete account: confirm | Signed out; USB backups untouched |

## 4. Responsive / window checks

- Resize window down to 640×480: no clipped content, page scrolls vertically.
- 900×640 default and maximized: header nav wraps (`Migrate My Files Pricing FAQ Settings Sign in`) without overlap.
- Long lists (AppPicker, My Files) scroll inside their containers.
- Keyboard: Enter submits password/sign-in forms; buttons show focus states.

## 5. Regression (run before every demo)

1. `cargo test` — all pass.
2. `npm run build` — tsc + vite clean.
3. `npm audit --omit=dev` — zero vulnerabilities.
4. Launch `npm run tauri dev` — single Ferry window, no devtools popup, no console errors.
5. Smoke: Welcome → DriveSelect lists real removable drives (or honest empty state).

## 6. Known gaps (not testable yet)

- **Boot has never been verified.** The bootloader is extracted from the
  downloaded ISO itself (no bundled binary), and the code is verified against
  the real Ubuntu 24.04.2 ISO's layout — but no USB Ferry produced has been
  booted on real firmware or in a UEFI VM yet. This is the biggest untested
  risk in the project.
- Ferry requires administrator rights (embedded manifest); `npm run tauri dev`
  must be launched from an **elevated** terminal, or `cargo run` cannot spawn
  the elevated binary.
- Windows MCT downloads are a deliberate cut, not a bug; Ubuntu direct download
  only. Windows entries appear greyed out in the OS picker.
- Chromium `Login Data` won't decrypt post-reinstall (DPAPI); passwords need browser sync.
- Cloud Backup requires `assist-server` to be running with a B2 master key
  configured (see `assist-server/README.md`).
