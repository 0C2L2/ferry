# Ferry MVP — Delivery Plan

## 1. Verdict

**Yes, the MVP is deliverable.** Not as a website — as a Windows desktop app,
which is what the repo already is. Every MVP capability maps to a real,
proven mechanism. Nothing in `mvp.md` requires research breakthroughs, only
engineering and one hardware test pass. The two genuinely hard spots
(Windows direct ISO, Secure Boot) each have a fallback that still ships a
real product.

## 2. Route decision: web vs. app vs. other

| Route | Verdict | Why |
|---|---|---|
| Pure web app | **No.** | Browsers cannot partition/format USB drives, write bootloaders, walk `C:\Users`, read the registry, or run `netsh`/`winget`/`driverquery`. The core function is impossible in a browser tab. |
| Desktop app (Tauri, current) | **Yes — this is the route.** | Native disk, registry, and process access; 5–10 MB portable binary; WebView2 already on Win10/11. Switching stacks now would burn time for zero gain. |
| Scripts / other | **No.** | A PowerShell bundle could technically do the steps but is unusable by the target user (non-technical). Rejected on product grounds. |
| Web companion | **Yes, supporting only.** | Marketing, Pricing/FAQ/docs, and later cloud account/billing. Never the engine. |

**Decision: ship Ferry as a Tauri desktop app (NSIS installer + portable EXE on the USB), with the website as companion only.**

## 3. Capability-by-capability feasibility (vs. `mvp.md`)

| # | MVP capability | Possible? | How |
|---|---|---|---|
| 1 | List removable USB drives | Yes | `GetDriveTypeW` + free space (built) |
| 2 | Partition FAT32 boot + exFAT data | Yes | Scripted `diskpart`, admin process (built, needs hardware test) |
| 3 | Ubuntu ISO direct + checksum | Yes | `releases.ubuntu.com` + SHA256SUMS resume download (built) |
| 4 | Windows 10/11 ISO | **Yes, with a spike** | See §5: time-boxed MCT-API work, fallback ladder below |
| 5 | Bootloader on FAT32 partition | Yes | Done differently: extract the ISO's own signed shim/GRUB onto FAT32 (Rufus-style) — no bundled GRUB, Secure Boot stays on |
| 6 | Scan/copy/verify personal files | Yes | Walkdir + SHA-256 manifest + verify gate (built, streaming) |
| 7 | Browser data (Chrome/Edge/Brave/Firefox) | Yes | File copy (built); Chrome passwords DPAPI-limited — message honestly, rely on sync |
| 8 | Wi-Fi export/import | Yes | `netsh` both directions (built, degrades gracefully) |
| 9 | App/driver inventory + 3-tier picker | Yes | Registry + winget + pnputil/driverquery + Store pass (built, installs working) |
| 10 | AES-256 backup container | Yes | Argon2id → AES-GCM chunked stream (built) |
| 11 | Restore to `Restored/`, Wi-Fi, summary | Yes | Decrypt → verify → copy → reimport (built) |
| 12 | Portable Ferry.exe on the USB | Yes | Ship release portable EXE alongside installer; Win10/11 carry WebView2 inbox |
| 13 | Admin elevation | Yes | App manifest `requireAdministrator` |

## 4. Delivery phases (in order — each ends in something testable)

### Phase A — Backup core lock-in (code complete, needs hardware proof)
- Run the full backup → verify → encrypt chain against a real USB on a real PC.
- Confirm: manifest matches, `Backup.enc` opens only with the password, wrong password fails clean.
- Exit criteria: test-plan B6/B7 rows pass on hardware.

### Phase B — USB engine (the determinative phase)
1. Bootloader = the ISO's signed EFI files extracted onto the FAT32 partition (`disk/bootloader.rs`, already built). No GRUB binary to obtain.
2. `prepare_usb` → copy bootloader → copy portable `Ferry.exe` → copy ISO, all on the two partitions.
3. Boot-test the USB on real hardware (not the build machine), Secure Boot ON.
4. Exit criteria: USB boots to the Ubuntu installer on test hardware.

### Phase C — OS acquisition
1. Ubuntu path: already built; verify checksum flow against a live download once.
2. Windows path, time-boxed spike: reproduce the Media Creation Tool download endpoint.
   - If the spike succeeds → implement + checksum per MCT metadata.
   - If it stalls → fallback ladder (§5), no schedule slip: Ubuntu-first launch.
3. Exit criteria: at least one Windows and the Ubuntu path download verified ISOs, or the fallback is formally adopted.

### Phase D — Restore + picker hardening
- Restore walkthrough on a second machine/VM from a Phase-B USB.
- AppPicker install-all queue, driver list display, restore summary review.
- Exit criteria: test-plan R1–R6 pass.

### Phase E — Signing + release artifacts
- EV code-signing certificate; sign installer + portable EXE + embedded bootloader.
- `app/test-plan.md` regression gate green; tag v0.1.0.
- Exit criteria: SmartScreen-clean install on a fresh machine.

### Phase F — Companion web presence (parallel, never blocking)
- Marketing page reusing the in-app Pricing/FAQ copy; docs; download hosting.
- Account system arrives with Cloud Backup billing (paid per `business-model.md`).

## 5. Decision points and fallbacks (decided now, not later)

1. **Windows ISO stalls** → Launch v0.1 as Ubuntu-first + Windows-via-MCT-manual (Ferry opens the official Microsoft download page and validates a user-provided ISO by published hash). The product still delivers its core promise; Windows-direct lands in v0.2.
2. **Secure Boot rejects the bootloader** → Document Secure Boot-off as the supported path (already the spec's position). No bypass tooling in v0.1.
3. **WebView2 missing on a target** → Ship the portable EXE with a bootstrapper note; Win10 1809+/Win11 carry it inbox, so this is an edge case, not a blocker.
4. **USB too small** → Clear pre-flight error with required size (already computed at scan); paid Cloud Backup covers the overflow case.

## 6. Explicitly deferred (not v0.1, no apology)
Cloud Backup billing/checkout (flow works; charging not wired), Store-app edge cases beyond inventory, cross-OS app-equivalent suggestions, multiboot, VM boot-test, macOS/Linux hosts, TPM bypass.

## 7. Definition of done
The MVP success metric in `mvp.md` §Success metric, executed on real hardware:
plug in → pick OS → password → backup + download + USB prep → wipe + reinstall
→ run Ferry from USB → files in `Restored/`, Wi-Fi reconnected, picker working —
without losing a file and without the user understanding partitions.

## 8. Needed from you (the only external dependencies)
1. One spare USB drive + one test PC (or VM) for Phases A/B/D.
2. The Ubuntu-first vs. Windows-direct call if the §5 spike stalls.
3. Budget approval for the EV certificate at Phase E.
