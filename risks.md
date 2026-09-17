# Risks and Guardrails

## The one that needs a hard guardrail, not a warning label

Picking the wrong physical disk is the most common way tools in this category destroy data. Since Ferry combines "write to this disk" and "this disk holds the only backup" on one device, that mistake is doubly expensive. Non-negotiable:

- Don't unlock "erase and install" until the backup on that same drive is checksum-verified.
- Show the actual disk model/size during selection, never just "Disk 2."

## Antivirus false positives are a category trait, not a bug

Every tool that does raw, low-level disk writes to make something bootable gets periodically flagged by some antivirus engine as a "potentially unwanted application" — the disk-level behavior itself resembles what malware does. This has happened to Rufus repeatedly for over a decade despite it being open source, signed, and used by millions. Plan for this operationally: code-sign every build, keep source auditable, have a whitelisting explainer ready.

## Malvertising / fake download sites

Covered in depth in `app-reinstall-picker.md` — the reason the app-picker never live-searches for a download link.

## Legal / licensing groundwork

- **Never redistribute official OS images.** Fetch fresh from the vendor's own servers per request, every time — never cache or mirror an ISO.
- **Windows digital licenses survive a wipe.** They're tied to the Microsoft account / hardware ID, not the hard drive, and reactivate automatically after a clean reinstall on the same hardware and edition. Worth stating plainly in onboarding — it's probably the user's second-biggest fear after losing files.
- **TPM 2.0 / Secure Boot bypass** (for installing Windows 11 on hardware Microsoft calls ineligible) is a deliberate fork, not a default decision — high demand given the Windows 10 EOL wave, but explicitly against Microsoft's own guidance and more likely to draw antivirus or store-review scrutiny.
