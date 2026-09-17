# Architecture

## USB partition layout

Modern OS install images (especially Windows) often contain a single file over 4 GB, and FAT32 — the filesystem most PC firmware natively boots from — cannot hold a file that large. Two existing tools already solve this; Ferry's layout borrows from both:

- **Rufus's approach**: a large NTFS partition holding the actual OS files, plus a small FAT32 partition at the end containing a bootloader (UEFI:NTFS) whose only job is letting FAT32-only firmware jump into the NTFS partition.
- **Ventoy's approach**: a small hidden boot partition, plus one large partition (exFAT by default, reformattable) that holds ISO files directly and uncompressed — Ventoy's own bootloader reads inside the ISO at boot time, so there's no 4 GB ceiling at all.

Ferry's planned layout follows Ventoy's shape, reused for two jobs at once:

- **Partition 1 (boot)**: ~32 MB, FAT32. Holds the bootloader only.
- **Partition 2 (data)**: the remainder, exFAT. Holds both the OS image (`.iso`) and a `Backup/` folder with the user's personal files, side by side.

> [!NOTE]
> exFAT is chosen for Partition 2 because it is natively supported on Windows, macOS, and modern Linux kernels, has no 4 GB file-size ceiling, and is readable by most PC firmware without extra drivers.

> [!IMPORTANT]
> Secure Boot compatibility must be tested per hardware/ISO combination — even mature tools document "Secure Boot off" as a fallback for stubborn hardware. Ferry should not promise universal Secure Boot support at MVP.

## Backup and restore philosophy

- Every file is backed up with its **full original path** recorded alongside it.
- On restore, that path is rebuilt *inside* a `Restored/` folder placed on the new desktop rather than injected into the new OS's real folders. Example: `C:\Users\John\Documents\taxes\2023.pdf` becomes `Restored/C_drive/Users/John/Documents/taxes/2023.pdf`.
- This works identically whether the restore target is the same OS or a completely different one, because the logic never tries to guess where something "should" go in the new system.
- Windows drive letters and reserved path characters (`:`, `\`, `/`) require a small normalization pass since they are not valid folder names on every filesystem.

## No installed-app restoration, by design

Byte-copying installed programs from one Windows install to another is unreliable — it breaks for driver-dependent software, anything tied to a hardware ID, or anything with a service expecting specific system state. Ferry never attempts it. Instead, see [`app-reinstall-picker.md`](app-reinstall-picker.md).

## Cloud overflow (when the USB isn't big enough)

When personal data exceeds what's left on the USB after the OS image is written, Ferry offers to park the overflow in the cloud temporarily, backed by Backblaze B2:

- **Storage cost**: ~\$6–7 / TB / month. For a typical overflow of 50 GB this is well under \$1 for a one-month bridge.
- **Egress cost**: downloads are free up to 3× whatever is stored that month — a full one-time restore comfortably fits inside that allowance.
- **Positioning**: a **migration bridge**, not permanent backup. Upload before the wipe, download after restore, then let it expire. (Backblaze's own consumer product sells unlimited ongoing backup at \$9/month — Ferry is not competing there.)
- **Encryption**: files are encrypted client-side before upload because the backup may include browser-saved passwords and product keys. The encryption key never leaves the user's machine.
