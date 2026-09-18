# Extensions (Beyond MVP)

These are validated ideas that belong in a future roadmap. None are blockers for the first ship. They are listed roughly in order of user value and implementation feasibility.

> [!NOTE]
> Backup partition encryption, browser data backup, and Wi-Fi profile export were originally listed here but have been **promoted to MVP scope** — see [`mvp.md §2 and §5`](mvp.md).

## High value — natural next features

- **~~Backup partition encryption~~** *(moved to MVP)* — A USB carrying tax documents, photos, saved passwords, and product keys is a physical object that can be lost or stolen. A password-protected AES-256 encrypted container makes a lost stick a non-event. See [`mvp.md §5`](mvp.md).
- **~~Browser data backup~~** *(moved to MVP)* — Bookmarks and saved logins, exported via each browser's native format (Chrome: `Bookmarks` JSON + `Login Data` SQLite; Firefox: `places.sqlite` + `logins.json`). Treated as personal data, not "an app," so it fits Ferry's philosophy. See [`mvp.md §2`](mvp.md).
- **~~Wi-Fi profile export / import~~** *(moved to MVP)* — Save and restore known network SSIDs and keys so the user is not retyping every remembered password after the wipe. On Windows, `netsh wlan export profile` already does this in one command. See [`mvp.md §2`](mvp.md).
- **License-key recovery** — Scan the same registry areas the app-detection pass already touches for common product keys (precedent: Belarc Advisor, ProduKey). Store the report locally alongside the app checklist. Keys are sensitive: keep them in the encrypted backup container.
- **Hardware / driver preflight check** — Record Wi-Fi adapter, GPU, and printer hardware IDs before the wipe; flag known driver gaps on the target OS (especially Linux) before committing. Helps the user avoid a post-wipe surprise of "my Wi-Fi doesn't work." The driver inventory already collected at MVP (see [`mvp.md §4`](mvp.md)) seeds this feature.


## Medium value — useful but deferrable

- **Multiboot menu** — One USB stick carrying several staged OS choices, Ventoy-style. Matches the original "let the user pick which OS" vision but adds complexity to the partition layout and boot chain.
- **Boot-test in a VM** — Verify that a prepared drive actually boots (Universal USB Installer already does this with an embedded VM) before the user wipes their only PC. High confidence value; the engineering lift is significant.
- **Skip what's already safe elsewhere** — Detect files already inside an active cloud-sync folder (Dropbox, OneDrive, Google Drive) and skip duplicating them onto the USB, reducing required USB space. Requires reading each sync client's config to know what is actually synced vs. just a placeholder.
- **Priority-ordered restore** — Bring back Documents and small personal files first, defer large media libraries. Useful on slow connections or when time is short after reinstall.

## Lower priority — nice-to-have polish

- **Backup-time and restore-time report** — Total size, item counts, apps and keys found, anything skipped and why, plus a hash-comparison "receipt" confirming nothing got corrupted in transit. Builds trust but adds polish time.
