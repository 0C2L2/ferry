# Ferry — Plan to Complete the Project

This is the execution plan for turning the current codebase into a shippable
MVP (per [`mvp.md`](mvp.md)) and beyond. It turns [`../STATUS.md`](../STATUS.md)'s
"what to do next" list into ordered, actionable work with owners, decisions
needed, and a definition of done for each phase. Update this file as phases
close — it should always reflect what's actually left, not what was left
when it was written.

**Snapshot at time of writing (2026-09-19):** backup/restore engine, encryption,
inventory, browser/Wi-Fi capture, migration-aware scanning, and cloud (B2)
upload are implemented and passing `cargo test` (22/22), `clippy -D warnings`,
and `tsc`. Cloud Backup is free (no tiers/subscription anywhere in the app —
see `business-model.md`). Windows download (MCT) is cut from this build;
Ubuntu is the only supported target for now. The bootable-USB blocker below
has a real implementation as of today, pending a boot test on real hardware
or a UEFI VM — the one thing that could not be verified inside this session.

---

## Phase 0 — Decisions needed before work starts

These are product calls, not engineering ones. Nothing downstream should be
built on a guess here.

| # | Decision | Resolution |
|---|---|---|
| D1 | Bootloader source | **Resolved 2026-09-19**: no bundled binary at all. Ferry extracts `/EFI/boot/` and `/boot/grub/` directly from the already-downloaded, checksum-verified OS ISO and writes a small `grub.cfg` that loopback-boots that same ISO from the data partition. The bootloader on the USB is always the OS vendor's own signed one. See Phase 1. |
| D2 | Windows download path for v0.1 | **Resolved 2026-09-19**: cut. Ubuntu-only for this build; Windows entries stay visible in the OS picker but greyed out as "automated download not available in this build" (already implemented in `OSSelect.tsx`) rather than removed, so the gap stays honest instead of hidden. |
| D3 | Code-signing certificate | Still open — see Phase 4. |
| D4 | Cloud Backup exposure in v0.1 | **Resolved 2026-09-19**, revised same day: ship it, free, and **Ferry-managed** — the user never creates their own B2 account/bucket/key. Ferry's own B2 master key lives only in `assist-server` (never in the desktop app); the app gets a disposable, backup-scoped credential minted per request. See `business-model.md` and `architecture.md`. |

---

## Phase 1 — Bootable USB (blocker) — implemented 2026-09-19, pending boot test

Goal: a USB Ferry prepares actually boots into the OS installer on UEFI
hardware. Without this there is no MVP, regardless of everything else working.

**What changed and why:** the erase/partition step (`disk/partition.rs`) used
to run *after* backup and OS download had already written to the drive —
`diskpart clean` would have destroyed both. It now runs immediately after
drive selection (new `PartitionUsb.tsx` step, right after `DriveSelect`),
before anything else touches the drive, and returns the actual assigned
`FERRY_BOOT`/`FERRY_DATA` drive letters (resolved by volume label, since
diskpart's `assign` does not guarantee any particular letter) rather than
reusing the stale pre-partition letter. Backup, Cloud Backup, download, and
the bootloader step all now write to the returned `data_letter`.

`disk/bootloader.rs` was rewritten to extract the bootloader from the
downloaded ISO instead of expecting a bundled binary:

1. Mount the ISO already sitting on the data partition (Windows'
   `Mount-DiskImage`, via a temp `.ps1` file with positional args — never
   string-interpolated, to keep arbitrary path content from being
   reinterpreted as script).
2. Copy `/EFI/boot/` and `/boot/grub/` onto the FAT32 boot partition.
3. Fail closed with a clear error if `boot/grub/loopback.cfg` isn't present
   (a different release/flavor could lay out its boot files differently).
4. Write a `grub.cfg` that loads the `exfat`/`iso9660`/`loopback` modules,
   `search`es the data partition for the ISO by filename, loopback-mounts it,
   and `source`s the ISO's own `loopback.cfg` (which expects a `$iso_path`
   variable — confirmed by inspection, see below).
5. Always dismount the ISO, even on failure.

**Verified, not guessed:** the actual `os-sources.json` Ubuntu 24.04.2 ISO
was downloaded (first 100 MB — the boot metadata lives early in the file)
and mounted for real during this session. Confirmed present: Canonical-signed
`bootx64.efi`/`grubx64.efi`/`mmx64.efi` under `/EFI/boot/`, and
`/boot/grub/loopback.cfg` containing exactly the `${iso_path}`-based
`casper/vmlinuz` menu entries this code's `grub.cfg` is written to drive.
`exfat.mod`/`fat.mod` are present among the ISO's own GRUB modules, confirming
GRUB can read the exFAT data partition once `insmod exfat` runs.

**What's still unverified:** whether the assembled USB actually boots. This
requires a real reboot — either a spare machine or a UEFI-enabled VM
(QEMU/VirtualBox + OVMF firmware; not installed in this environment, so it
could not be attempted here). Do this before any live demo.

**Definition of done:** a freshly-prepared Ubuntu USB boots on at least one
real UEFI machine or VM and reaches the Ubuntu installer. Windows targets are
out of scope for this build per D2.

---

## Phase 2 — OS download completeness (blocker) — resolved 2026-09-19 (cut)

Per **D2**, Windows MCT is cut for this build rather than implemented.
`OSSelect.tsx` already showed Windows entries as greyed out ("automated
download not available in this build") since `os-sources.json` has `url:
null` for both Windows entries — no UI change was needed, this was already
honest. Ubuntu 24.04.2/22.04.5 (direct URL, checksum-verified) are the only
working sources.

**Still open, tracked as v0.2 work, not started:**
1. Reverse/implement the Media Creation Tool manifest API flow (product
   edition GUID lookup → ESD manifest → ESD or ISO URL).
2. Extend `download/sources.rs`'s `SourceType::MicrosoftMct` from an unused
   enum variant into a real resolver, mirroring the existing `DirectUrl`
   checksum-then-resume pattern in `download/fetch.rs`.
3. Update `os-sources.json` with real Windows 10/11 entries + checksum
   verification (MCT ESD files ship SHA1/SHA256 in the manifest).

**Definition of done (for this build):** every OS listed as available in the
UI downloads and checksum-verifies successfully end-to-end — true today for
Ubuntu — with nothing advertised that doesn't work.

---

## Phase 3 — USB self-hosting

Goal: restore can run entirely from the USB with no re-download, per
`mvp.md` §4 and the "Ferry portable exe" checklist items in `STATUS.md`.

1. Add a build step that copies a portable `Ferry.exe` (self-contained, no
   installer) onto the USB data partition during `prepare_usb`.
2. Verify the portable exe launches directly from a FAT32/exFAT USB on a
   freshly-installed OS with no other Ferry install present.
3. Confirm restore flow (`restore/detect.rs` → decrypt → verify → copy) works
   when launched this way, before any network connection is confirmed.
4. Resolve **D4** for Cloud Backup: if shipping, make sure the "Extra
   Careful" B2 upload path is reachable from this portable-exe restore
   context too (or explicitly scope it to backup-time only).

**Definition of done:** unplug network, boot new OS, run Ferry from USB,
restore files successfully with zero downloads.

---

## Phase 4 — Distribution readiness

1. **CI first green run**: `.github/workflows/ci.yml` exists (cargo test +
   clippy gate, npm build + audit) but has never executed on a runner —
   push/PR it and fix whatever the runner environment surfaces that local
   dev didn't (Windows-specific toolchain paths, PowerShell command
   availability, etc.).
2. **Code signing** (per **D3**): acquire the certificate, wire signing into
   the release build (`tauri build` + signtool), and confirm SmartScreen/AV
   don't flag a signed build the way they would an unsigned one (`risks.md`
   anticipates this).
3. **Release pipeline**: tag-triggered build producing the installer + the
   portable exe (Phase 3) as build artifacts.
4. **`os-sources.json` update job**: since it's data-driven by design, add a
   scheduled/manual job to refresh Ubuntu LTS point-release URLs and
   checksums without a full app release.

**Definition of done:** a tagged commit produces a signed, downloadable
installer via CI with no manual steps.

---

## Phase 5 — Integration & QA pass

1. Run every case in `app/test-plan.md` B/R/S sections on real hardware,
   not just unit tests — this is explicitly the biggest untested-integration
   risk per `STATUS.md` §9.
2. Cover the migration-plan adversarial cases from
   [`migration-scan-plan.md`](../app/migration-scan-plan.md) §9 on a real
   multi-user Windows machine, not just synthetic paths.
3. Exercise the Cloud Backup path against a real B2 bucket: interrupted
   upload resume, wrong credentials, bucket-not-found, and large (multi-GB)
   file behavior.
4. Re-run the full `mvp.md` ship checklist top to bottom and flip every box
   that's now genuinely true (several already are, per `STATUS.md`, but the
   checklist itself hasn't been re-walked since new features landed).

**Definition of done:** `test-plan.md` fully executed with results recorded,
no unchecked ship-checklist item without a documented reason.

---

## Backlog (post-MVP, do not start before Phases 1–5)

Pulled from `STATUS.md` §9 and `extensions.md` — sequenced last on purpose:

- Cross-OS app-equivalent suggestions in the reinstall picker.
- Game-save / cloud-placeholder awareness in migration scanning
  (`migration-scan-plan.md` §10).
- `get_drive_model` batching (currently shells out per-drive; slow with many
  drives attached).
- Automated UI test coverage (currently backend unit tests + manual only).
- Update `business-model.md` and `mvp.md`'s "cut from MVP" table now that
  Cloud Backup has moved from "does not exist" to "implemented, pending D4."

---

## How to use this file

- Treat Phase 0's decisions as prerequisites — don't let engineering start
  drift ahead of an unmade product call.
- Phases 1 and 2 are the actual MVP blockers; everything else is sequenced
  after them on purpose.
- When a phase's definition of done is met, check it off here and update
  `STATUS.md`'s comparison table in the same commit so the two documents
  never drift apart again.
