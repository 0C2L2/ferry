# App Reinstall Picker

Ferry detects what was installed before the wipe and gives the user a checklist with install buttons — but never restores the app itself, only points to where to get it again.

## Detection

- **Windows**: registry Uninstall keys (`HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall` and `HKCU` equivalent, plus the `Wow6432Node` path for 32-bit apps) expose `DisplayName`, `DisplayVersion`, `Publisher`, and often a `URLInfoAbout` field pointing at the vendor's homepage. Microsoft Store apps require a separate pass via `Get-AppxPackage` (PowerShell) or the `PackageManager` WinRT API.
- **macOS**: scan `/Applications` bundles' `Info.plist` for `CFBundleName`, `CFBundleIdentifier`, and `CFBundleShortVersionString`.
- **Linux**: ask the package manager directly (`dpkg --get-selections`, `rpm -qa`, `pacman -Q`) — it already knows exactly what was installed and from which repository.

## Resolving a name to a trustworthy source — three tiers

1. **Tier 1 (best)**: the app exists in a package manager's own catalog (winget, Homebrew Cask, apt / dnf / pacman). The button doesn't link anywhere — it runs that manager's install command directly. Winget's catalog is a public, moderated repository where every manifest ships an installer URL with a SHA-256 hash and passes an automated SmartScreen check before merging — Ferry inherits that trust pipeline instead of building its own.
2. **Tier 2**: no package-manager entry exists, but a usable lead does — the registry's own `URLInfoAbout` field, or a small curated, versioned lookup table Ferry maintains for common unpackaged software (e.g. Steam, Zoom, Slack installers whose official download URLs are stable and well-known).
3. **Tier 3 (no confident match)**: say so, plainly, unlinked. **Never fall back to a live web search and grab the top result** — security researchers have found 70+ lookalike domains impersonating popular Windows apps, some ranking highly in search results for months, occasionally serving the real download at first before swapping it for malware after they had earned trust. A Tier 3 row must never look as confident as a Tier 1/2 row.

> [!CAUTION]
> The Tier 3 "no match" state is a feature, not a failure. Displaying an unverified link is strictly worse than displaying nothing.

## UI shape

A checklist grouped by category (Browsers, Developer Tools, Creative, Utilities, Other), each row tagged with its resolution tier, checkboxes per row, "Select all" per group, and a single "Install selected" action at the bottom that queues installs sequentially.

## Cross-OS handling

When restoring onto a different OS than the backup came from, Tier 1 will fail for most OS-specific software. That is the moment to surface a "closest equivalent" suggestion instead of a dead row — for example:

| Backed-up app | Suggested equivalent |
|---|---|
| Photoshop | GIMP, Krita |
| Microsoft Office | LibreOffice |
| iTunes | Rhythmbox, Clementine |
| Notepad++ | Kate, gedit |

These suggestions come from the same curated lookup table used for Tier 2, not from a live search.

## Versioning

Always default to installing the *latest* version from the source, not the old detected version. State this explicitly in the UI so the user is not surprised if an app looks slightly different after restore.
