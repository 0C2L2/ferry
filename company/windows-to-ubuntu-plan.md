# Windows 11 → Ubuntu 24.04: Migration Plan

The concrete plan for Ferry's headline journey: a Windows 11 user moves to
Ubuntu 24.04, keeps their personal files, and gets an honest answer about
their apps on the other side.

Scope of this document: classifying what's on the Windows machine, deciding
what crosses the gap, restoring it on Ubuntu, and turning the Windows app list
into Linux recommendations. It builds on
[`../app/migration-scan-plan.md`](../app/migration-scan-plan.md) (how folder
selection works) and [`app-reinstall-picker.md`](app-reinstall-picker.md) (the
existing three-tier source rules).

---

## 0. The blocker: Ferry cannot run on Ubuntu — RESOLVED 2026-09-23

Everything below is downstream of one fact. `app/src-tauri` has **no**
`cfg(windows)` gating anywhere, hard dependencies on the `winreg` and
`windows` crates, and shells out to `diskpart`, `netsh`, `winget`,
`driverquery`, `pnputil` and PowerShell across eleven modules. `restore/copy.rs`
resolves the destination from `%USERPROFILE%`.

Ferry did not compile for Linux, let alone run there: the user reached their
new Ubuntu desktop holding a USB that nothing could open, and the restore half
of the product did not exist for this journey. Fixed on 2026-09-23 — the
library now cfg-gates its Windows-only modules and ships a `ferry-restore`
CLI.

Nothing else in this plan matters until that's fixed, so it's Phase 1 — and
**Phase 1 has now landed**; see the status note at the end of §1.

---

## 1. Phase 1 — A restore path that runs on Ubuntu — DONE (untested on real hardware)

### Decision: a separate small Linux CLI, not a Linux port of Ferry

Porting the GUI means cfg-gating or reimplementing disk enumeration,
partitioning, Wi-Fi, registry and winget — none of which restore needs. What
restore actually needs is narrow:

1. Find `Backup.enc` + `backup.salt` on the USB
2. Ask for the password, derive the key (Argon2id), decrypt (chunked AES-GCM)
3. Verify checksums from `manifest.json`
4. Copy into `Restored/` on the desktop
5. Print a summary

Steps 2–4 are already written and already portable — `crypto/keygen.rs`,
`crypto/stream.rs`, `crypto/decrypt.rs` and `restore/copy.rs` use `std::fs`,
`sha2`, `aes-gcm` and `zip`, with no Windows API calls. The Windows-specific
parts are the *neighbours*, not the logic.

So: **a second binary in the same crate**, `ferry-restore`, sharing those
modules. No GUI, no Tauri, no reimplementation of the container format (which
must never be reimplemented — two parsers of one crypto format is how you ship
a silent corruption bug).

```
[[bin]]
name = "ferry-restore"
path = "src/restore_cli/main.rs"
```

Work required:

- `cfg`-gate the Windows-only modules out of the library for non-Windows
  targets, so `crypto/` and `restore/` can compile alone.
- Replace the `%USERPROFILE%\Desktop` lookup with a platform split. On Linux
  use `xdg-user-dir DESKTOP`, falling back to `$HOME/Desktop`: Ubuntu localises
  the desktop folder (`~/Bureau`, `~/Escritorio`), and hardcoding `Desktop`
  silently creates a second, wrong folder.
- Drop Wi-Fi reimport on Linux. `netsh` XML is meaningless to NetworkManager;
  attempting a conversion is a guess. Report the saved networks as a list the
  user can retype instead — see §3.
- Build it on the existing Ubuntu CI runner (`.github/workflows/ci.yml`), not
  by cross-compiling from Windows.
- Copy the binary onto the USB during `prepare_usb`, next to the Windows
  `Ferry.exe` that Phase 3 of the completion plan already owes.

**Definition of done:** boot the Ubuntu live session or a fresh install, run
`./ferry-restore` from the USB, enter the password, and land the files in
`~/Desktop/Restored/` with a printed summary — no network, no install.

### Status, 2026-09-23 — built, not yet run

Everything above is implemented and both targets lint clean under
`-D warnings` (`x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`):

- Windows-only modules gated; `crypto`, `restore::copy`, `safety`, `types` and
  a new shared `hashing` module compile for Linux.
- `src/restore_cli/main.rs` finds the backup (argument, next to the binary, or a
  walk of `/media`, `/mnt`, `/run/media`), prompts for the password with
  `rpassword`, decrypts, verifies and copies, then prints a summary.
- `restore_to_desktop` takes a progress callback so the GUI and CLI share one
  verify-then-copy implementation.
- `xdg-user-dir DESKTOP` with `$HOME/Desktop` then `$HOME` as fallbacks.
- CI job `restore-cli` on `ubuntu-latest` builds and uploads the binary.

Two things surfaced while building it that the plan had not anticipated:

- **`zip`'s default features pull in C libraries** (bzip2, zstd, lzma) that
  broke cross-compilation. Ferry only ever writes deflate, so
  `default-features = false` both fixed the build and dropped a native dep.
- **`cfg(windows)` in `build.rs` describes the host, not the target.** Build
  scripts compile for the build machine, so it still fired while
  cross-compiling and `tauri-build` panicked. The target is read from
  `CARGO_CFG_TARGET_OS` instead.

**Still unproven:** the CLI has never executed on an actual Ubuntu machine
against a real backup. Compiling is not the same as working.

> [!IMPORTANT]
> Ubuntu mounts exFAT read-only in some configurations and the USB is
> exFAT. Restore only reads, so this is survivable, but it must be tested —
> and `ferry-restore` needs a clear error rather than a permissions stack
> trace if the mount is missing entirely.

---

## 2. What's actually on a Windows 11 machine, and what crosses

### What Ferry does today

Folder-based selection over the top level of `C:\Users\<name>\`, with Desktop,
Documents, Downloads, Pictures, Music and Videos pre-ticked, everything else
listed with its measured size, and cache/temp paths excluded by default.
Browser profiles, Wi-Fi profiles and the app/driver inventory are captured by
separate passes. That mechanism is sound and stays.

### What it misses for this journey

Folder selection alone doesn't answer "will this be any use on Ubuntu?".
Four categories need explicit handling:

| Category | Examples | Crosses? | Plan |
|---|---|---|---|
| **Portable user data** | Documents, photos, music, video, PDFs, source code, `.ssh`, `.gitconfig` | Yes, cleanly | Back up and restore as-is. This is the core promise. |
| **Windows-only artifacts** | `.exe`, `.msi`, `.lnk` shortcuts, `.reg` exports, `Thumbs.db`, `desktop.ini` | No | Back up (they're the user's files; silently dropping them violates "never guess"), but **report** them as "won't be usable on Ubuntu" in the summary. Shortcuts especially: a Desktop full of `.lnk` files restores as a folder of dead links unless we say so. |
| **App-owned data outside the big six** | Outlook `.pst`, Thunderbird profiles, game saves under `AppData`, Steam `userdata` | Sometimes | Not auto-selected today. Surface the known ones as opt-in extras in the migration plan (§2.2). |
| **Cloud placeholders** | OneDrive files-on-demand stubs | **Dangerous** | Detect and handle explicitly — see below. |

### 2.1 OneDrive placeholders — a real data-loss trap — DONE 2026-09-23

Windows 11 ships OneDrive on by default with Files On-Demand. A placeholder
looks like a normal file with a normal size in Explorer, but its content lives
in the cloud and the local file is a sparse stub. Copy it blind and the user
gets a file of the right name and the wrong (empty) contents — and only finds
out after the wipe.

This is exactly the failure Ferry exists to prevent, so it cannot be left to
chance:

- Detect via the file attributes `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` and
  `FILE_ATTRIBUTE_OFFLINE`.
- Before backup, tell the user plainly how many files are cloud-only and how
  much they'd need to download to include them.
- Offer: hydrate them (trigger download), skip them, or continue and accept
  that they're placeholders.
- Never copy a placeholder while reporting it as a backed-up file.

**Implemented 2026-09-23.** `backup/scan.rs` checks
`FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`, `FILE_ATTRIBUTE_RECALL_ON_OPEN` and
`FILE_ATTRIBUTE_OFFLINE` on every file and returns `cloud_only_count` /
`cloud_only_bytes` in `ScanResult`. `ExcludeReview.tsx` shows an amber warning
with the count and size, and tells the user to right-click → "Always keep on
this device" and rescan — while the drive is still intact. Placeholders stay
in the file list rather than being silently dropped; the user decides.

### 2.2 Opt-in extras for a Linux move

Add a short, curated list of known app-data locations to the migration plan
screen, unticked by default, each with its measured size and a one-line note
on whether it's useful on Ubuntu:

- `AppData\Roaming\Thunderbird` → Thunderbird on Ubuntu reads the same profile
  format. Genuinely portable.
- `AppData\Local\Microsoft\Outlook\*.pst` → no Outlook on Linux; importable
  into Thunderbird with a converter. Worth keeping, needs work.
- `.ssh`, `.gnupg`, `.aws`, `.kube` → portable, and painful to lose.
- Steam `userdata` → cloud-synced for most titles; keep as a safety net.

Curated and human-verified, same rule as the app picker: no guessing, and
anything unknown stays unlisted rather than speculatively included.

---

## 3. Wi-Fi, credentials, and things that honestly don't cross

State these up front in the UI, not in a summary after the wipe:

- **Wi-Fi profiles.** Exported from `netsh` as Windows XML. NetworkManager
  doesn't read it. Ferry should show the SSID list (and keys, since it already
  has them with `key=clear`) as a printable list rather than pretending to
  migrate them.
- **Browser passwords.** Chromium's `Login Data` is DPAPI-bound to the Windows
  install and will not decrypt after the wipe — already documented, and it does
  not improve on Linux. The honest answer remains browser sync.
- **Windows licence keys.** Irrelevant on Ubuntu; don't offer to carry them.
- **Fonts, drivers, registry.** Not portable. Drivers stay informational.

---

## 4. Apps: Windows list → Ubuntu recommendations

### What exists

`inventory/apps.rs` (registry uninstall keys) + `inventory/store.rs`
(`Get-AppxPackage`) produce the list. `inventory/picker.rs` resolves each entry
to Tier 1 (exact winget match), Tier 2 (`URLInfoAbout`, labelled unverified) or
Tier 3 (nothing). **All three tiers are Windows answers** — winget IDs and
vendor `.exe` links are useless on Ubuntu.

### The distinction that must not be blurred

Two different things get called "the Linux version":

1. **The same application, on Linux** — Firefox, VLC, VS Code, Steam, Spotify,
   Discord, OBS, Blender, GIMP-if-they-already-used-GIMP.
2. **A different application that does a similar job** — Photoshop → GIMP or
   Krita; MS Office → LibreOffice; Notepad++ → Kate.

Telling a user "Photoshop → GIMP" as if it were the same app is the kind of
quiet dishonesty Ferry is built against. Every recommendation is labelled as
**Same app** or **Alternative**, and alternatives say what's lost.

### Proposed Linux tiers

| Tier | Meaning | Source shown |
|---|---|---|
| **L1 — Same app, official Linux build** | Vendor ships Linux | The vendor's documented install method (apt repo, `.deb`, Flatpak, snap) from a curated table |
| **L2 — Same app, in Ubuntu's archive** | In `main`/`universe` | `sudo apt install <pkg>` |
| **L3 — Alternative** | No Linux build; a well-known substitute exists | Named substitute + one line on what differs |
| **L4 — No answer** | No Linux version and no substitute Ferry will vouch for | Say so plainly. Windows-only games, niche vendor tools, hardware utilities |

### Where the mapping lives and runs — DONE 2026-09-23

A curated `linux-equivalents.json` in the repo, keyed on the registry
`DisplayName` with exact matching — the same fail-closed rule
`curated-apps.json` already uses for Tier 2.

**Resolve it on Windows, at backup time**, and write a plain
`Backup/linux-apps.md` alongside `apps.json`. Rationale: it needs no network
and no package-index queries on the Linux side, it survives even if
`ferry-restore` never runs, and the user can open it in any text editor or
phone. Querying apt/snap/flatpak live from Ubuntu would be a second resolver to
build, test and keep honest, for no benefit the static table doesn't give.

Seed the table with the top ~50 Windows apps by install share; everything else
lands in L4 and says so. A partial table that admits its gaps beats a complete
one built on guesses.

**Implemented 2026-09-23.** `app/linux-equivalents.json` holds ~45 curated
entries; `inventory/linux.rs` resolves the inventory against it and
`inventory/save.rs` writes `Backup/linux-apps.md` next to `apps.json`. The
output is grouped into *Same app, available on Linux* / *Different app, similar
job* / *No Linux version* / *Not in Ferry's list*, so "same app" and
"alternative" can never be read as the same claim — there are tests asserting
Photoshop resolves as an alternative and GIMP as the same app. Unlisted apps
are named and explicitly not guessed at. Writing the checklist never fails the
backup.

Matching is a case-insensitive prefix compare against the curated key, because
registry `DisplayName` values carry version and locale suffixes ("Mozilla
Firefox (x64 en-US)") that exact matching would miss. Both sides are
human-written, so it stays deterministic rather than fuzzy.

---

## 5. Sequencing

1. ~~**Phase 1 — `ferry-restore` for Linux.**~~ **Done 2026-09-23** (builds and
   lints for Linux; never executed on real hardware).
2. ~~**OneDrive placeholder detection.**~~ **Done 2026-09-23** (detected by file
   attribute, warned before the wipe; untested against a live OneDrive).
3. ~~**`linux-equivalents.json` + `linux-apps.md`.**~~ **Done 2026-09-23**
   (~45 curated entries, written at backup time).
4. **Windows-only artifact reporting** (`.lnk`, `.exe` counts in the summary) —
   not started.
5. **Opt-in app-data extras** (§2.2) — not started.
6. **Wi-Fi/credential honesty copy** in the UI — not started.

Items 4–6 are all "tell the user the truth before the irreversible step" and
remain cheap. The expensive, load-bearing ones are done; what is *not* done is
running any of it on a real Ubuntu machine.

Items 2, 4 and 6 are all "tell the user the truth before the irreversible
step", and they're cheap. If the schedule slips, they should survive the cut
before the app recommendations do.

---

## 6. What this plan deliberately does not do

- **No Linux GUI.** `ferry-restore` is a CLI. A GUI is a second frontend to
  maintain for one screen.
- **No Windows→Linux settings migration.** Themes, keybindings, and app
  preferences don't map, and pretending otherwise produces a broken desktop.
- **No automatic app installation on Ubuntu.** Ferry hands over a checklist
  with verified sources; running `apt install` on the user's behalf means
  Ferry owns the consequences of a bad package choice.
- **No `.pst` conversion.** Named as a known gap in §2.2 rather than
  half-implemented.
