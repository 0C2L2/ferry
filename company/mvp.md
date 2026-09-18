# Ferry — MVP Definition

This document defines exactly what ships in the first public version of Ferry, what is explicitly cut, and what the ship checklist looks like. Everything else lives in [`extensions.md`](extensions.md).

---

## Goal of the MVP

Ship the smallest version of Ferry that solves the real problem end-to-end for the most common case:

> **A non-technical Windows 10 user wants to reinstall Windows or switch to Linux without losing their personal files, and they have a USB drive large enough to hold both the OS image and their files.**

If this person can run Ferry start-to-finish without data loss, the MVP has succeeded.

---

## What's in scope

### 1. USB drive preparation

- Detect all connected removable/external drives and list them with model name, capacity, and available space.
- Partition the USB: a small FAT32 boot partition (~32 MB) and a large exFAT data partition filling the rest.
- Download the selected OS image directly from the vendor's official server (see §3 below) into the data partition.
- Write a bootloader to the boot partition so the drive is bootable on UEFI systems.
- Show real-time progress (download %, write %, estimated time remaining).
- **Copy the Ferry installer itself onto the USB** so the user has it ready to run on the new OS without needing to re-download it.

> [!IMPORTANT]
> **Hard safety rule**: the "erase and write" step is locked until the backup (§2) is checksum-verified. The sequence is always: backup → verify → erase → write. This order cannot be changed by the user.

### 2. Personal-file backup

- **Scan all personal folders automatically.** Default to the entire `C:\Users\<username>\` directory tree — every subfolder is included unless the user explicitly excludes it. The user does not need to manually select folders one by one.
- Excluded by default (safe to skip): OS-managed cache and temp folders (`AppData\Local\Temp`, browser caches, `Thumbs.db`, `.DS_Store`). These are excluded silently with a count shown in the summary.
- Copy everything into `Backup/` on the exFAT data partition, preserving the original folder structure exactly. Each folder keeps its original name. Example: `C:\Users\John\Documents\taxes\` becomes `Backup\Users\John\Documents\taxes\` on the USB, and is restored to `Restored\Users\John\Documents\taxes\` on the new desktop.
- After copying, compute and record a SHA-256 checksum for every backed-up file in a `Backup/manifest.json` alongside the files.
- Verify each checksum against the source before declaring the backup complete. Do not unlock the erase step until verification passes.
- Show a summary: total files, total size, any files that failed (with reason), and a count of items skipped (with reasons).

> [!NOTE]
> The "exclude" list is user-reviewable before the backup starts. A user who wants to include something normally excluded can add it back.

**Browser data** — backed up as personal data, not as an app:

- **Chrome / Edge / Brave**: copy `Bookmarks` (JSON) and `Login Data` (SQLite) from the browser profile directory.
- **Firefox**: copy `places.sqlite` (bookmarks + history) and `logins.json` (saved passwords) from the Firefox profile directory.
- Saved passwords are sensitive — this is why **backup encryption (§6) is mandatory**, not optional, in this MVP.
- Browser data is restored into `Restored\BrowserData\<BrowserName>\` on the new desktop, not silently injected back into the browser. The user imports it manually from there.

**Wi-Fi profiles** — backed up and restored automatically:

- On Windows, `netsh wlan export profile folder="Backup\WiFi\" key=clear` exports all saved network profiles including SSIDs and keys in XML format.
- At restore time, `netsh wlan add profile filename="<file>"` reimports each profile — the user's known networks reconnect automatically without retyping passwords.
- Wi-Fi profiles are also sensitive (they contain network keys) and are covered by the backup encryption in §6.

### 3. OS image download

MVP supports exactly these OS sources at launch:

| OS | Source | Notes |
|---|---|---|
| Windows 11 | Microsoft's official Media Creation Tool API | Same endpoint the Media Creation Tool uses |
| Windows 10 | Microsoft's official Media Creation Tool API | For hardware that fails Win 11 requirements |
| Ubuntu LTS (current) | `releases.ubuntu.com` | SHA-256 checksum verified against Ubuntu's GPG-signed hash file |

All downloads must be verified with the vendor's published checksum before the file is used. If verification fails, the file is deleted and the user is notified.

> [!NOTE]
> Adding more OS options post-MVP is low risk and high value. The source list is data-driven (a versioned JSON manifest), not hardcoded, so new entries can ship as config updates without a full app release.

### 4. App and driver inventory

At backup time, Ferry scans for two things and saves them alongside the file backup:

**Apps — `Backup/apps.json`**
- Registry paths: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*`, `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*`, and `HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*`.
- Capture per app: `DisplayName`, `DisplayVersion`, `Publisher`, `URLInfoAbout`.

**Drivers — `Backup/drivers.json`**
- Run `driverquery /v /fo csv` to enumerate all installed drivers with name, description, type, and start mode.
- Cross-reference with Device Manager's export (`pnputil /enum-drivers`) to capture third-party OEM drivers (GPU, Wi-Fi, printers) separately from inbox Windows drivers.
- Save driver name, INF file name, provider, and version. This list is informational at MVP — it helps the user know what to reinstall manually, and seeds the post-MVP hardware/driver preflight feature.

At restore time (post-wipe, on the new OS), the user runs Ferry from the USB drive directly — **no separate download needed**. Ferry detects `Backup/apps.json` and `Backup/drivers.json` automatically and opens the reinstall picker.

> [!IMPORTANT]
> Ferry must be runnable directly from the USB on the new OS before any internet connection is confirmed. Ship Ferry as a self-contained portable `.exe` on the USB, not just an installer.

Reinstall picker behaviour at MVP:
- **Tier 1**: resolve against winget catalog (`winget search` / manifest lookup). Show "Install via winget" button.
- **Tier 2**: fall back to the `URLInfoAbout` field from the registry, if present and not empty. Show "Get from vendor site" link, clearly labelled as unverified.
- **Tier 3**: show "No confident source found" — grey row, no link.

### 5. Backup encryption

Because the backup now includes browser saved passwords and Wi-Fi keys, encrypting the USB data partition is **mandatory** — not a future extension.

**Approach**: AES-256 encrypted archive wrapping the entire `Backup/` folder. VeraCrypt volumes are more robust but require VeraCrypt to be installed on the new OS before restore; an encrypted archive (e.g. 7-zip AES-256 with a `.7z` container) is self-contained and openable by Ferry's own built-in decryption without any extra software.

**User flow**:
1. At the start of backup, Ferry prompts the user to set an encryption password. It is not recoverable — losing it means losing access to the backup.
2. Ferry warns the user clearly: *"Write this password down somewhere safe. We cannot recover it for you."*
3. The `Backup/` folder is encrypted into `Backup.enc` on the USB data partition. The unencrypted `Backup/` folder is deleted after encryption is verified.
4. The `manifest.json` checksum file is stored inside the encrypted container so verification also requires the password.
5. At restore time, Ferry prompts for the password before any files are decrypted or copied.

> [!CAUTION]
> A lost encryption password = total data loss. The UI must make this impossible to miss. Consider requiring the user to type the password twice at creation, and again to confirm they have written it down before proceeding.

> [!NOTE]
> The `Ferry.exe` portable copy on the USB, and the OS `.iso` file, remain outside the encrypted container — the user can still boot from the USB and run Ferry without entering the password first. Only the `Backup.enc` file requires the password.

### 6. Restore walkthrough

After the new OS is installed and the user plugs in the USB drive, they run Ferry directly from the USB. Ferry should:

1. Detect `Backup.enc` on the USB automatically.
2. Prompt the user for their encryption password and decrypt `Backup.enc` into a temporary in-memory working area (not written to disk unencrypted).
3. Verify checksums of all backed-up files (from the decrypted `manifest.json`) before copying anything.
4. Copy files into `Restored/` on the new desktop, rebuilding the original folder structure with original folder names intact.
5. Reimport Wi-Fi profiles automatically via `netsh wlan add profile` — the user's networks are reconnected without any manual step.
6. Open the app and driver reinstall picker (`apps.json` + `drivers.json`).
7. Show a final summary: files restored, files skipped, verification failures (if any), Wi-Fi networks restored.

---

## What's explicitly cut from MVP

| Feature | Reason cut | Lives in |
|---|---|---|
| Cloud overflow storage | Requires account system and backend infrastructure; post-MVP | [`business-model.md`](business-model.md) |
| License-key recovery | Nice-to-have; registry scanning for keys is already partially done | [`extensions.md`](extensions.md) |
| Hardware / driver preflight | High value for Linux switchers; driver inventory (above) seeds this | [`extensions.md`](extensions.md) |
| Multiboot (multiple ISOs) | Complex boot chain; out of scope | [`extensions.md`](extensions.md) |
| Boot-test in VM | Significant engineering lift | [`extensions.md`](extensions.md) |
| macOS and Linux as source OS | Backup/detection logic differs per OS; Windows first | Future |
| TPM 2.0 / Secure Boot bypass | Opt-in only, not default; legal and AV scrutiny risk | [`risks.md`](risks.md) |

---

## Platform target

- **Host OS at MVP**: Windows 10 and Windows 11 only.
- **Target OS (what can be installed)**: Windows 10, Windows 11, Ubuntu LTS.
- **UI framework**: to be decided (Tauri + web frontend recommended for cross-platform headroom; Electron is also viable but heavier).
- **Installer**: must be code-signed (see [`risks.md`](risks.md)). Provide both an `.exe` installer and a portable `.exe` that runs from the USB without installation.

---

## Ship checklist

### Safety
- [ ] Disk selection greys out all non-removable drives
- [ ] Disk model and capacity shown at selection (never just a drive letter)
- [ ] "Erase and write" is locked behind successful backup verification
- [ ] Second explicit confirmation required before erase ("I understand this will erase the drive permanently")
- [ ] Backup manifest (`manifest.json`) written and verified before erase step unlocks
- [ ] Restore verification: checksums re-checked before copying to new OS

### Backup
- [ ] Full `C:\Users\<username>\` tree scanned by default
- [ ] Default exclude list applied silently (Temp, caches) with count shown
- [ ] User can review and modify the exclude list before backup starts
- [ ] Skipped files listed with reason in summary
- [ ] `Backup/drivers.json` written via `driverquery` + `pnputil` output

### Browser data
- [ ] Chrome/Edge/Brave `Bookmarks` and `Login Data` files located and copied
- [ ] Firefox `places.sqlite` and `logins.json` files located and copied
- [ ] Browser files placed in `Restored\BrowserData\<BrowserName>\` at restore (not silently injected)
- [ ] Browser profile directories correctly resolved across Windows user account names

### Wi-Fi profiles
- [ ] `netsh wlan export profile` runs successfully and writes all profiles to `Backup\WiFi\`
- [ ] All exported profiles reimported at restore via `netsh wlan add profile`
- [ ] Success / failure count shown in restore summary

### Encryption
- [ ] Password set at start of backup (with confirmation field)
- [ ] User shown explicit "write this down" warning before proceeding
- [ ] `Backup/` encrypted into `Backup.enc` (AES-256) after copy and verify
- [ ] Unencrypted `Backup/` folder deleted after `Backup.enc` is verified
- [ ] `Ferry.exe` and `.iso` remain outside the encrypted container (accessible without password)
- [ ] At restore: password prompt appears before any file is decrypted or read
- [ ] Wrong password shows a clear error, does not silently produce corrupt output

### Downloads
- [ ] All OS downloads fetched directly from vendor servers (no proxy, no cache)
- [ ] Downloaded image checksum verified against vendor-published hash
- [ ] Failed checksum causes file deletion and user notification (no silent retry with corrupt file)
- [ ] Download can be resumed if interrupted (range requests or chunked download)

### USB self-hosting
- [ ] Ferry portable `.exe` copied onto the USB data partition during preparation
- [ ] Portable `.exe` runs without installation on a fresh Windows install
- [ ] Restore walkthrough launchable directly from USB before any network connection

### App & driver picker
- [ ] Winget catalog lookup working for Tier 1
- [ ] `URLInfoAbout` fallback working for Tier 2
- [ ] Tier 3 rows visually distinct from Tier 1/2 (grey, no link, no install button)
- [ ] "Install via winget" actually runs the correct winget command
- [ ] No live web search ever used to resolve an app source
- [ ] Driver list displayed separately from app list, clearly labelled as informational

### UX
- [ ] Progress shown for: backup copy, backup verify, encryption, OS download, USB write, decryption, restore copy, restore verify
- [ ] Error states have plain-language messages (no raw exception text shown to user)
- [ ] Final backup summary: file count, total size, skipped count with reasons, failures listed by name with reason
- [ ] Final restore summary: restored count, skipped count, Wi-Fi networks restored, verification failures

### Legal / distribution
- [ ] Build is code-signed
- [ ] No OS image cached or redistributed — every download is fresh from vendor
- [ ] First-run onboarding mentions: Windows digital licenses survive a reinstall (activation is automatic)
- [ ] Edition-matching warning: user is shown the detected Windows edition so they can download the matching one

---

## Success metric for MVP

A non-technical user can:
1. Plug in a USB drive.
2. Pick an OS.
3. Set an encryption password and let Ferry back up their files, download the OS, and prepare the USB.
4. Wipe and reinstall (manually, using the USB Ferry prepared).
5. Plug the USB back in, enter their password, run Ferry directly from the USB, and get all their personal files onto the new desktop — with their Wi-Fi reconnected automatically.
6. Use the app and driver picker to reinstall what they care about.

…without losing a single file and without needing to understand what a partition is.

