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

> [!IMPORTANT]
> **Hard safety rule**: the "erase and write" step is locked until the backup (§2) is checksum-verified. The sequence is always: backup → verify → erase → write. This order cannot be changed by the user.

### 2. Personal-file backup

- Present a folder picker defaulting to: `Desktop`, `Documents`, `Downloads`, `Pictures`, `Music`, `Videos`, and any additional paths the user adds.
- Copy selected folders into `Backup/` on the exFAT data partition, preserving the full original path (with drive-letter normalization for cross-OS compatibility: `C:` → `C_drive`).
- After copying, compute and record a SHA-256 checksum for every backed-up file in a `Backup/manifest.json` alongside the files.
- Verify each checksum against the source before declaring the backup complete. Do not unlock the erase step until verification passes.
- Show a summary: total files, total size, any files that failed (with reason).

### 3. OS image download

MVP supports exactly these OS sources at launch:

| OS | Source | Notes |
|---|---|---|
| Windows 11 | Microsoft's official Media Creation Tool API | Same endpoint the Media Creation Tool uses |
| Windows 10 | Microsoft's official Media Creation Tool API | For hardware that fails Win 11 requirements |
| Ubuntu LTS (current) | `releases.ubuntu.com` | SHA-256 checksum verified against Ubuntu's GPG-signed hash file |
| Fedora (current) | `dl.fedoraproject.org` | Checksum verified against Fedora's signed hash |
| Linux Mint (current) | `mirrors.linuxmint.com` | Checksum verified against Mint's signed hash |

All downloads must be verified with the vendor's published checksum before the file is used. If verification fails, the file is deleted and the user is notified.

> [!NOTE]
> Adding more OS options post-MVP is low risk and high value. The source list is data-driven (a versioned JSON manifest), not hardcoded, so new entries can ship as config updates without a full app release.

### 4. App inventory (read-only in MVP)

At backup time, scan the Windows registry Uninstall keys and produce a plain list of installed applications:
- Registry paths: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*`, `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*`, and `HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*`.
- Capture: `DisplayName`, `DisplayVersion`, `Publisher`, `URLInfoAbout`.
- Save the list as `Backup/apps.json`.

At restore time (post-wipe, on the new OS), the user opens Ferry and loads `apps.json` to get the reinstall picker. **The reinstall picker UI is in scope for MVP** — it must be usable on day one because it is the only app-restoration path.

Reinstall picker behaviour at MVP:
- **Tier 1**: resolve against winget catalog (`winget search` / manifest lookup). Show "Install via winget" button.
- **Tier 2**: fall back to the `URLInfoAbout` field from the registry, if present and not empty. Show "Get from vendor site" link, clearly labelled as unverified.
- **Tier 3**: show "No confident source found" — grey row, no link.

### 5. Restore walkthrough

After the new OS is installed and the user plugs in the USB drive, Ferry (re-run on the new OS) should:

1. Detect the `Backup/` folder and `manifest.json` automatically.
2. Verify checksums of all backed-up files before copying anything.
3. Copy files into `Restored/` on the new desktop, rebuilding the original path structure.
4. Open the app reinstall picker (`apps.json`).
5. Show a final summary: files restored, files skipped, verification failures (if any).

---

## What's explicitly cut from MVP

| Feature | Reason cut | Lives in |
|---|---|---|
| Cloud overflow (Backblaze B2) | Significant backend infrastructure; not needed if USB is large enough | [`business-model.md`](business-model.md) + post-MVP |
| Backup partition encryption | Important but adds UX complexity; deferred unless browser backup ships | [`extensions.md`](extensions.md) |
| Browser data backup | Requires encryption to be safe; both deferred together | [`extensions.md`](extensions.md) |
| Wi-Fi profile export | Useful but not core to the file-safety mission | [`extensions.md`](extensions.md) |
| License-key recovery | Nice-to-have; registry scanning for keys is already partially done | [`extensions.md`](extensions.md) |
| Hardware / driver preflight | High value for Linux switchers; post-MVP | [`extensions.md`](extensions.md) |
| Multiboot (multiple ISOs) | Complex boot chain; out of scope | [`extensions.md`](extensions.md) |
| Boot-test in VM | Significant engineering lift | [`extensions.md`](extensions.md) |
| macOS and Linux as source OS | Backup/detection logic differs per OS; Windows first | Future |
| TPM 2.0 / Secure Boot bypass | Opt-in only, not default; legal and AV scrutiny risk | [`risks.md`](risks.md) |

---

## Platform target

- **Host OS at MVP**: Windows 10 and Windows 11 only.
- **Target OS (what can be installed)**: Windows 10, Windows 11, Ubuntu, Fedora, Linux Mint.
- **UI framework**: to be decided (Tauri + web frontend recommended for cross-platform headroom; Electron is also viable but heavier).
- **Installer**: must be code-signed (see [`risks.md`](risks.md)). Provide both an `.exe` installer and a portable `.zip`.

---

## Ship checklist

### Safety
- [ ] Disk selection greys out all non-removable drives
- [ ] Disk model and capacity shown at selection (never just a drive letter)
- [ ] "Erase and write" is locked behind successful backup verification
- [ ] Second explicit confirmation required before erase ("I understand this will erase the drive permanently")
- [ ] Backup manifest (`manifest.json`) written and verified before erase step unlocks
- [ ] Restore verification: checksums re-checked before copying to new OS

### Downloads
- [ ] All OS downloads fetched directly from vendor servers (no proxy, no cache)
- [ ] Downloaded image checksum verified against vendor-published hash
- [ ] Failed checksum causes file deletion and user notification (no silent retry with corrupt file)
- [ ] Download can be resumed if interrupted (range requests or chunked download)

### App picker
- [ ] Winget catalog lookup working for Tier 1
- [ ] `URLInfoAbout` fallback working for Tier 2
- [ ] Tier 3 rows visually distinct from Tier 1/2 (grey, no link, no install button)
- [ ] "Install via winget" actually runs the correct winget command
- [ ] No live web search ever used to resolve an app source

### UX
- [ ] Progress shown for: backup copy, backup verify, OS download, USB write, restore copy, restore verify
- [ ] Error states have plain-language messages (no raw exception text shown to user)
- [ ] Final backup summary: file count, total size, failures listed by name with reason
- [ ] Final restore summary: restored count, skipped count, verification failures

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
3. Let Ferry back up their files, download the OS, and prepare the USB.
4. Wipe and reinstall (manually, using the USB Ferry prepared).
5. Plug the USB back in, run Ferry, and get all their personal files onto the new desktop.
6. Use the app picker to reinstall the apps they care about.

…without losing a single file and without needing to understand what a partition is.

