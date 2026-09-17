# Risks and Guardrails

## Wrong-disk selection — the one risk that needs a hard guardrail

Picking the wrong physical disk is the most common way tools in this category destroy data. Since Ferry combines "write to this disk" and "this disk holds the only backup" on one device, that mistake is doubly expensive. These protections are non-negotiable:

- **Don't unlock the "erase and write" step** until the backup on that same drive has been checksum-verified byte-for-byte.
- **Show the actual disk model and size** during selection — never just "Disk 2" or a drive letter.
- **Require an explicit second confirmation** ("I understand this will erase the drive") before writing, even after the backup is verified.
- **Grey out all system disks** in the device list; only removable/external drives are selectable.

> [!CAUTION]
> There is no undo for a disk wipe. The sequence must be: backup → verify → confirm → erase. Skipping or reordering any step is not allowed.

## Antivirus false positives are a category trait, not a bug

Every tool that performs raw, low-level disk writes to produce a bootable drive gets periodically flagged by some antivirus engine as a "potentially unwanted application" — the disk-level behavior resembles what malware does. This has happened to Rufus repeatedly for over a decade despite it being open source, code-signed, and used by millions. Plan for this operationally:

- Code-sign every release build with a trusted certificate (EV code-signing certificate recommended for SmartScreen reputation).
- Keep the source code publicly auditable.
- Prepare a whitelisting explainer (for both end-users and AV vendors) before the first public release.

## Malvertising / fake download sites

Covered in depth in [`app-reinstall-picker.md`](app-reinstall-picker.md) — the reason the app-picker never performs a live web search for a download link.

## Legal and licensing groundwork

- **Never redistribute official OS images.** Fetch fresh from the vendor's own servers per request, every time — never cache or re-host an ISO. Caching even temporarily may constitute redistribution under Microsoft's and Canonical's terms.
- **Windows digital licenses survive a wipe.** They are tied to the Microsoft account or hardware ID, not the hard drive, and reactivate automatically after a clean reinstall on the same hardware and the same edition. State this plainly during onboarding — it is probably the user's second-biggest fear after losing files.
- **Edition matching matters.** Restoring a Windows 10 Home license onto a Windows 11 Pro image will not activate. Ferry should detect and display the backed-up edition so the user downloads the matching one.
- **TPM 2.0 / Secure Boot bypass** (for installing Windows 11 on hardware Microsoft calls ineligible) is a deliberate opt-in fork, not a default — high demand given the Windows 10 EOL wave, but explicitly against Microsoft's own guidance and more likely to draw antivirus or app-store-review scrutiny. Treat as a post-MVP decision.
