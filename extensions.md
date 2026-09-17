# Extensions (Beyond MVP)

These are validated ideas that belong in a future roadmap. None are blockers for the first ship. They are listed roughly in order of user value and implementation feasibility.

## High value — natural next features

- **Backup partition encryption** — A USB carrying tax documents, photos, saved passwords, and product keys is a physical object that can be lost or stolen. A password-protected encrypted container (e.g. VeraCrypt volume or AES-256 encrypted archive) makes a lost stick a non-event. *This should be bumped to MVP if browser-data backup ships.*
- **Browser data backup** — Bookmarks and saved logins, exported via each browser's native format (Chrome: `Bookmarks` JSON + `Login Data` SQLite; Firefox: `places.sqlite` + `logins.json`). Treated as personal data, not "an app," so it fits Ferry's philosophy — but it makes encryption above effectively mandatory.
- **Wi-Fi profile export / import** — Save and restore known network SSIDs and keys so the user is not retyping every remembered password after the wipe. On Windows, `netsh wlan export profile` already does this in one command.
- **License-key recovery** — Scan the same registry areas the app-detection pass already touches for common product keys (precedent: Belarc Advisor, ProduKey). Store the report locally alongside the app checklist. Keys are sensitive: keep them in the encrypted backup partition if that feature exists.
- **Hardware / driver preflight check** — Record Wi-Fi adapter, GPU, and printer hardware IDs before the wipe; flag known driver gaps on the target OS (especially Linux) before committing. Helps the user avoid a post-wipe surprise of "my Wi-Fi doesn't work."

## Medium value — useful but deferrable

- **Multiboot menu** — One USB stick carrying several staged OS choices, Ventoy-style. Matches the original "let the user pick which OS" vision but adds complexity to the partition layout and boot chain.
- **Boot-test in a VM** — Verify that a prepared drive actually boots (Universal USB Installer already does this with an embedded VM) before the user wipes their only PC. High confidence value; the engineering lift is significant.
- **Skip what's already safe elsewhere** — Detect files already inside an active cloud-sync folder (Dropbox, OneDrive, Google Drive) and skip duplicating them onto the USB, reducing required USB space. Requires reading each sync client's config to know what is actually synced vs. just a placeholder.
- **Priority-ordered restore** — Bring back Documents and small personal files first, defer large media libraries. Useful on slow connections or when time is short after reinstall.

## Lower priority — nice-to-have polish

- **Backup-time and restore-time report** — Total size, item counts, apps and keys found, anything skipped and why, plus a hash-comparison "receipt" confirming nothing got corrupted in transit. Builds trust but adds polish time.
