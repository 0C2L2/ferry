# Ferry

Ferry is a free desktop app that makes switching or reinstalling a computer's operating system safe for people who aren't technical. It downloads an official OS image, writes a bootable USB drive, and — on the same drive — backs up the user's personal files so nothing gets lost in the process.

## The problem

Windows 10 reached end of support on October 14, 2025, and a large share of otherwise-fine PCs can't meet Windows 11's hardware requirements (TPM 2.0, Secure Boot). That leaves a lot of people facing a choice — upgrade, switch to Linux, or keep running an unsupported OS — and most are afraid of the process because they don't know how to safely back up first.

## What Ferry actually does

1. **Backs up personal files** to the same USB drive that will hold the OS installer, preserving each file's original full path.
2. **Downloads the OS** the user picks, always from the vendor's own official servers — never rehosted or cached.
3. **Writes one bootable USB** combining a small boot partition with a large data partition that holds both the OS image and the backup — the same two-partition trick tools like Ventoy and Rufus already use to get past the FAT32 4GB file-size limit on modern OS images.
4. **After the OS installs**, the user's files land in a `Restored/` folder on the new desktop, rebuilt from their original path — not silently placed back into OS folders, and never auto-injected into the new system.
5. **Installed apps are never restored as binaries.** Instead, Ferry hands back a checklist of what was installed, with install buttons that lead to verified official sources — see `app-reinstall-picker.md`.

## Design principles that shape everything else

- **Never guess.** Every download — the OS image, an app's install source — comes from a verified, known-official location, never a live web search result.
- **Be honest about what didn't work.** If there's no confident source for an app, say so — don't paper over it with a guessed link.
- **Don't restore what can't be trusted.** No installed-app binaries or registry state ever get carried over; only personal files and a plain list of what was there.
- **The wipe step is the point of no return.** Every safeguard (backup verification, checksum checks) exists to protect that one moment.

## Docs in this repo

- `architecture.md` — USB partition layout, backup/restore philosophy, cloud overflow
- `app-reinstall-picker.md` — the three-tier app detection and install system
- `extensions.md` — feature ideas beyond MVP
- `business-model.md` — monetization plan
- `risks.md` — guardrails and legal/licensing groundwork

## Name

"Ferry" — it carries you and your files across to the new OS, the way an actual ferry carries passengers and cargo across water.
