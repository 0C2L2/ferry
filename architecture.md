# Architecture

## The USB layout

Modern OS install images (especially Windows) often contain a single file over 4GB, and FAT32 — the filesystem most PC firmware natively boots from — can't hold a file that large. Two existing tools already solve this, and Ferry's layout borrows from both:

- **Rufus's approach**: a large NTFS partition holding the actual OS files, plus a small FAT partition at the end containing a bootloader (UEFI:NTFS) whose only job is letting FAT-only firmware jump into the NTFS partition.
- **Ventoy's approach**: a small hidden boot partition, plus one large partition (exFAT by default, reformattable) that holds ISO files directly and uncompressed — Ventoy's own bootloader reads inside the ISO at boot time, so there's no 4GB ceiling at all.

Ferry's planned layout follows Ventoy's shape, reused for two jobs at once:

- **Partition 1**: small boot partition (Ventoy-style or custom).
- **Partition 2**: one large exFAT/NTFS partition holding both the OS image *and* a `Backup/` folder with the user's files, side by side.

Secure Boot compatibility needs to be tested on/off separately rather than promised universally — even mature tools still document Secure-Boot-off as a fallback for stubborn hardware/ISO combinations.

## The backup and restore philosophy

- Every file is backed up with its **full original path** recorded alongside it.
- On restore, that path is rebuilt *inside* a `Restored/` folder on the new desktop rather than injected into the new OS's real folders — e.g. `C:\Users\John\Documents\taxes\2023.pdf` becomes `Restored/C_drive/Users/John/Documents/taxes/2023.pdf`.
- This works identically whether the restore target is the same OS or a completely different one, because the logic never tries to guess where something "should" go in the new system.
- Windows drive letters and reserved characters (`:`) need a small normalization pass since they aren't valid folder names on every filesystem.

## No installed-app restoration, by design

Byte-copying installed programs from one Windows install to another is unreliable — it breaks for driver-dependent software, anything tied to a hardware ID, or anything with a service expecting specific system state. Ferry never attempts it. Instead, see `app-reinstall-picker.md`.

## Cloud overflow (when the USB isn't big enough)

When personal data exceeds what's left on the USB after the OS image, Ferry offers to park the overflow in the cloud temporarily, backed by Backblaze B2:

- Storage: roughly $6–7/TB/month.
- Downloads (restoring) are free up to 3x whatever is stored that month — a full one-time restore comfortably fits inside that allowance.
- Positioned as a **migration bridge**, not permanent backup: upload before the wipe, download after restore, then let it expire. (Backblaze's own consumer product already sells unlimited ongoing backup at $9/month — Ferry isn't trying to compete there.)
- Files are encrypted client-side before upload, since the backup may include browser-saved passwords and product keys.
