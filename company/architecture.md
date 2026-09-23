# Architecture

## USB partition layout

Modern OS install images (especially Windows) often contain a single file over 4 GB, and FAT32 — the filesystem most PC firmware natively boots from — cannot hold a file that large. Two existing tools already solve this; Ferry's layout borrows from both:

- **Rufus's approach**: a large NTFS partition holding the actual OS files, plus a small FAT32 partition at the end containing a bootloader (UEFI:NTFS) whose only job is letting FAT32-only firmware jump into the NTFS partition.
- **Ventoy's approach**: a small hidden boot partition, plus one large partition (exFAT by default, reformattable) that holds ISO files directly and uncompressed — Ventoy's own bootloader reads inside the ISO at boot time, so there's no 4 GB ceiling at all.

Ferry's layout is closer to Rufus's "ISO Image mode" than to Ventoy's:

- **Partition 1 (boot)**: FAT32, sized to the chosen OS image plus ~15% headroom. Holds the image's **extracted contents**, so the vendor's own signed `/EFI/boot/bootx64.efi` and `grub.cfg` boot directly. Ferry writes no boot configuration of its own. (Floor of 100 MB: FAT32 needs ~33.5 MB minimum, so a 32 MB partition cannot be formatted at all — Ventoy uses FAT16 at that size for the same reason.)
- **Partition 2 (data)**: the remainder, exFAT. Holds the `Backup/` folder (later `Backup.enc` + `backup.salt`) and the Ferry portable `.exe`.

The `.iso` is downloaded to the data partition, extracted onto the boot partition, then deleted — it would otherwise waste several GB the backup needs.

> [!NOTE]
> **Why not keep the ISO whole and loopback-boot it?** That is the standard dodge for FAT32's 4 GB per-file limit, and Ferry did it that way initially. Measuring the real Ubuntu 24.04.2 ISO showed its largest member is `casper/minimal.squashfs` at 1.69 GB — the limit never bound. The loopback scheme cost a hand-written `grub.cfg`, a dependency on GRUB loading its exFAT module, and partition-addressing assumptions, all on the one code path that cannot be verified without rebooting. Plain extraction removes all of it. If a future image *does* carry a >4 GB member, that image needs the loopback path (or NTFS + a UEFI:NTFS shim, as Rufus does) and the layout has to change with it.

> [!NOTE]
> exFAT is chosen for Partition 2 because it is natively supported on Windows, macOS, and modern Linux kernels, has no 4 GB file-size ceiling, and is readable by most PC firmware without extra drivers.

> [!IMPORTANT]
> Secure Boot compatibility must be tested per hardware/ISO combination — even mature tools document "Secure Boot off" as a fallback for stubborn hardware. Ferry should not promise universal Secure Boot support at MVP.

## Backup and restore philosophy

- Every file is backed up with its **full original path** recorded alongside it.
- On restore, that path is rebuilt *inside* a `Restored/` folder placed on the new desktop, with the original folder names preserved exactly. Example: `C:\Users\John\Documents\taxes\2023.pdf` becomes `Restored\Users\John\Documents\taxes\2023.pdf` on the new desktop.
- The drive prefix is stripped in the stored relative path (`C:\Users\...` → `Users/...`); the complete original path is always kept in `manifest.json`'s `original_path` field. Drive letters and reserved path characters (`:`, `\`, `/`) are therefore never used as folder names.
- This works identically whether the restore target is the same OS or a completely different one, because the logic never tries to guess where something "should" go in the new system.

## No installed-app restoration, by design

Byte-copying installed programs from one Windows install to another is unreliable — it breaks for driver-dependent software, anything tied to a hardware ID, or anything with a service expecting specific system state. Ferry never attempts it. Instead, see [`app-reinstall-picker.md`](app-reinstall-picker.md).

## Cloud backup (overflow and Extra Careful)

When personal data exceeds what's left on the USB after the OS image is written, or when the user wants a second encrypted cloud copy alongside the USB, Ferry offers Cloud Backup — the one paid feature (see [`business-model.md`](business-model.md)). It is hosted on Ferry's own Backblaze B2 account: **Ferry-managed**, not bring-your-own-B2. The user registers and signs in (no bucket or key to create), and gets back a single opaque backup ID (a UUID) to write down alongside their password.

**Why the desktop app never holds Ferry's B2 master key:** a distributed desktop binary can always have embedded secrets extracted from it (`strings`, a decompiler). If the app carried Ferry's real B2 credentials, every install would effectively leak them, and anyone could run up storage costs, fill the bucket with junk, or touch other users' data. So the master key lives only in `assist-server` (see `assist-server/src/services/b2admin.ts`), and the desktop app talks to that sidecar instead of to B2 directly for credentials.

Technical notes (`app/src-tauri/src/cloud/b2.rs` + `assist-server/src/services/b2admin.ts`):

- **Per-backup, disposable credentials.** For every upload, `assist-server` asks B2 to mint a brand-new Application Key scoped to exactly one bucket, one `namePrefix` (`<backup_id>/`), one capability (`writeFiles` for upload, `readFiles` for restore), and a short validity window (6h for upload, 30 min for restore). Even a leaked key can't touch any other backup, and it expires on its own. The desktop app authorizes with that scoped key directly against B2 for the actual file transfer — `assist-server`'s own bandwidth is never in the path for the bytes themselves, only the small "mint me a key" calls.
- Each backup lives at `<backup_id>/Backup.enc` and `<backup_id>/backup.salt` in the shared bucket — the ID is the only thing that needs to survive a lost USB; the password alone isn't enough without it, matching how the local salt file already works.
- **Backend: Backblaze B2.** Storage cost to Ferry: ~\$6–7/TB/month. Egress is **free** up to 3× whatever is stored that month — this is why B2, specifically, is the backend: a full one-time restore comfortably fits inside the free allowance, unlike GCS/S3 where egress alone would be real money per restore.
- Positioning: a **migration bridge**, not permanent backup. Once `restore_files` succeeds, the desktop app tells `assist-server` to delete both objects for that backup ID — Ferry doesn't hold onto a copy after the user has it back.
- The already-encrypted `Backup.enc` (+ `backup.salt`) is uploaded as-is via B2's Large File API (chunked, resumable); B2 never sees plaintext. The AES-256-GCM key is derived from the user's password and never transmitted anywhere — Ferry cannot read the contents of a user's cloud backup, even though it now hosts the storage.
