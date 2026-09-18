# Architecture

## USB partition layout

Modern OS install images (especially Windows) often contain a single file over 4 GB, and FAT32 — the filesystem most PC firmware natively boots from — cannot hold a file that large. Two existing tools already solve this; Ferry's layout borrows from both:

- **Rufus's approach**: a large NTFS partition holding the actual OS files, plus a small FAT32 partition at the end containing a bootloader (UEFI:NTFS) whose only job is letting FAT32-only firmware jump into the NTFS partition.
- **Ventoy's approach**: a small hidden boot partition, plus one large partition (exFAT by default, reformattable) that holds ISO files directly and uncompressed — Ventoy's own bootloader reads inside the ISO at boot time, so there's no 4 GB ceiling at all.

Ferry's planned layout follows Ventoy's shape, reused for two jobs at once:

- **Partition 1 (boot)**: ~32 MB, FAT32. Holds the bootloader only.
- **Partition 2 (data)**: the remainder, exFAT. Holds the OS image (`.iso`), the `Backup/` folder with the user's personal files, and the Ferry portable `.exe` — all side by side.

> [!NOTE]
> exFAT is chosen for Partition 2 because it is natively supported on Windows, macOS, and modern Linux kernels, has no 4 GB file-size ceiling, and is readable by most PC firmware without extra drivers.

> [!IMPORTANT]
> Secure Boot compatibility must be tested per hardware/ISO combination — even mature tools document "Secure Boot off" as a fallback for stubborn hardware. Ferry should not promise universal Secure Boot support at MVP.

## Backup and restore philosophy

- Every file is backed up with its **full original path** recorded alongside it.
- On restore, that path is rebuilt *inside* a `Restored/` folder placed on the new desktop, with the original folder names preserved exactly. Example: `C:\Users\John\Documents\taxes\2023.pdf` becomes `Restored\Users\John\Documents\taxes\2023.pdf` on the new desktop.
- This works identically whether the restore target is the same OS or a completely different one, because the logic never tries to guess where something "should" go in the new system.
- Windows drive letters and reserved path characters (`:`, `\`, `/`) require a small normalization pass since they are not valid folder names on every filesystem. `C:` becomes `C_drive`, etc.

## No installed-app restoration, by design

Byte-copying installed programs from one Windows install to another is unreliable — it breaks for driver-dependent software, anything tied to a hardware ID, or anything with a service expecting specific system state. Ferry never attempts it. Instead, see [`app-reinstall-picker.md`](app-reinstall-picker.md).

## Cloud backup (overflow and Extra Careful)

When personal data exceeds what's left on the USB after the OS image is written, or when the user wants a second encrypted cloud copy alongside the USB, Ferry offers Cloud Backup as a paid add-on. See [`business-model.md`](business-model.md) for plan details and pricing.

Technical notes:

**Individual plan — Backblaze B2 backend:**
- Storage cost: ~\$6–7/TB/month. Holding 200 GB for one month costs ~\$1.40.
- Egress cost: **free** up to 3× whatever is stored that month. A full one-time restore after a migration comfortably fits inside this allowance. This free-egress policy is why B2 is chosen for the Individual plan — on GCS or S3, a 200 GB restore would cost ~\$16–24 in egress alone, erasing the margin.
- Positioning: a **migration bridge**, not permanent backup. Upload before the wipe, download after restore, then let the cloud copy expire.

**Corporate / SMB plan — GCS, AWS S3, or Azure Blob backend:**
- Chosen for enterprise-grade SLA, global redundancy, and audit trails that business customers require.
- Egress is not free on these providers (~\$0.08–0.12/GB). Corporate plan pricing is subscription-based (not one-time) to account for this.

**Both plans:**
- Files are encrypted client-side (AES-256) before upload. The encryption key is derived from the user's password and never transmitted to Ferry's servers — Ferry cannot read the contents of a user's cloud backup.
