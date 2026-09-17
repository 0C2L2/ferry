# Extension Ideas (beyond MVP)

- **Multiboot menu** — one USB stick carrying several staged OS choices, Ventoy-style, matching the original "let the user pick which OS" framing.
- **Boot-test in a VM** — verify a prepared drive actually boots (Universal USB Installer already does this with an embedded VM) before the user wipes their only PC.
- **License-key recovery** — scan the same registry areas the app-detection pass already touches for product keys (precedent: Belarc Advisor, ProduKey), store the report locally alongside the app checklist.
- **Browser data backup** — bookmarks and saved logins, via each browser's native export format. Personal data, not "an app," so it fits Ferry's philosophy — but raises the case for encryption (below).
- **Backup partition encryption** — a USB carrying tax documents, photos, and now saved logins is a physical object that can be lost or stolen. A password-protected container makes a lost stick a non-event instead of a disaster.
- **Wi-Fi profile export/import** — save and restore known network SSIDs and keys so the user isn't retyping every remembered password.
- **Hardware/driver preflight check** — record Wi-Fi, GPU, and printer hardware before wiping, flag known driver gaps on the target OS (especially Linux) before committing.
- **Skip what's already safe elsewhere** — detect files already inside an active cloud-sync folder (Dropbox/OneDrive/Google Drive) and skip duplicating them onto the USB.
- **Priority-ordered restore** — bring back Documents and small personal files first, defer large media libraries.
- **Backup-time and restore-time report** — total size, item counts, apps and keys found, anything skipped and why, plus a hash-comparison "receipt" that nothing got corrupted.
