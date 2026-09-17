# App Reinstall Picker

Ferry detects what was installed before the wipe and gives the user a checklist with install buttons — but never restores the app itself, only points to where to get it again.

## Detection

- **Windows**: registry Uninstall keys (`HKLM`/`HKCU`, plus `Wow6432Node`) give DisplayName, DisplayVersion, Publisher, and often a `URLInfoAbout` field pointing at the vendor's homepage. Microsoft Store apps need a separate pass.
- **macOS**: scan `/Applications` bundles' `Info.plist`.
- **Linux**: ask the package manager directly (`dpkg`, `rpm`, `pacman`) — it already knows exactly where each package came from.

## Resolving a name to a trustworthy source — three tiers

1. **Tier 1 (best)**: the app exists in a package manager's own catalog (winget, Homebrew Cask, apt/dnf/pacman). The button doesn't link anywhere — it runs that manager's install command directly. Winget's catalog in particular is a public, moderated repo where every manifest ships an installer URL with a SHA256 hash and passes an automated SmartScreen check before merging — Ferry inherits that trust pipeline instead of building its own.
2. **Tier 2**: no package-manager entry, but a usable lead exists — the registry's own `URLInfoAbout` field, or a small curated, versioned lookup table Ferry maintains for common unpackaged software.
3. **Tier 3 (no confident match)**: say so, plainly, unlinked. **Never fall back to a live web search and grab the top result** — security researchers have found 70+ lookalike domains impersonating popular Windows apps, some ranking in search results for months, occasionally showing the *real* download link at first before swapping it for malware later once they'd earned trust. A Tier 3 row must never look as confident as a Tier 1/2 row.

## UI shape

A checklist grouped by category (browsers, dev tools, creative, utilities), each row tagged with which tier it resolved at, checkboxes, "select all" per group, one "Install selected" action.

## Cross-OS handling

When restoring onto a different OS than the backup came from, Tier 1 will fail for most OS-specific software. That's the moment to surface a "closest equivalent" suggestion (e.g. Photoshop → GIMP/Krita, Office → LibreOffice) instead of a dead row.

## Versioning

Always default to installing the *latest* version from the source, not the old detected version — worth stating explicitly in the UI.
