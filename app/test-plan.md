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
  `ferry.plannedTier`, `ferry.history`) — inspect via DevTools
  (`FERRY_DEVTOOLS=1`). Sign out to start a session clean.

## 3. Functional cases

### Auth & account

| ID | Steps | Expected |
|---|---|---|
| A1 | Sign up with invalid email / short password / mismatched state | Clear inline error, no account created |
| A2 | Sign up valid → header shows first name → reload app | Session persists, Account page shows Ferry Free |
| A3 | Sign out → sign in with wrong password | "Wrong email or password", stays signed out |
| A4 | Sign in correct → sign out | Header returns to "Sign in", session key removed |

### Pricing & plan choice

| ID | Steps | Expected |
|---|---|---|
| P1 | Open Pricing signed out, click a Cloud tier | Redirects to Account (sign-in) page |
| P2 | Signed in, click "Up to 200 GB" | Row shows "✓ planned"; Account page shows planned tier |
| P3 | Click the same tier again | Preference cleared |
| P4 | Click "Choose Corporate" signed in | "✓ Corporate planned" + no-charge note; visible on Account |
| P5 | Click "Planned — not available" button | Disabled; nothing happens (no fake checkout) |

### Backup wizard (needs spare USB)

| ID | Steps | Expected |
|---|---|---|
| B1 | Welcome → Back up, no USB inserted | "No removable USB drives found" |
| B2 | Insert USB → Rescan → select drive → Continue | Selection highlighted; OS step unlocks |
| B3 | OS picker | Ubuntu selectable; MCT options marked unavailable in this build |
| B4 | Exclude review | File count + size shown; add/remove extra exclude rescans |
| B5 | Password: short / mismatched / unchecked box | Blocked with plain-language error |
| B6 | Backup progress | All 6 stages complete; manifest noted; no erase yet |
| B7 | Download Ubuntu | Progress %, checksum verified; corrupt download deleted + error |
| B8 | Write step without checking the box | Erase button stays disabled |
| B9 | Check box → Erase & write | Partition/format stages pass; Done page with boot steps |
| B10 | My Files after B6 | New "Backup" record with drive, OS, file count, size |

### Safety rules (must all hold)

| ID | Check |
|---|---|
| S1 | System (non-removable) drives never appear in DriveSelect |
| S2 | Erase step unreachable without a verified backup in the same session (restart app → token gone) |
| S3 | Second explicit erase confirmation required before `diskpart clean` |
| S4 | Wrong backup password at restore → clear error, no partial/corrupt output |
| S5 | USB unplugged mid-copy → failure surfaces, no silent success |

### Restore & picker (needs prepared USB + password)

| ID | Steps | Expected |
|---|---|---|
| R1 | RestoreDetect with no Ferry USB | "No Ferry backup found" |
| R2 | Correct USB + wrong password | Decrypt fails cleanly |
| R3 | Correct password | Decrypt → verify → copy → Wi-Fi; summary counts; files under `Restored/` on desktop |
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

- No `assets/bootx64.efi`: bootloader step reports its absence honestly; USB is not yet bootable.
- Windows MCT downloads unimplemented (manifest has no URLs); Ubuntu direct download only.
- Chromium `Login Data` won't decrypt post-reinstall (DPAPI); passwords need browser sync.
- Cloud Backup, billing, and server auth do not exist; plan choice is a local preference.
