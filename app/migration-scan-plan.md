# Migration-Aware Scanning — Engineering Plan

How Ferry decides **what to back up** based on **where the user is going**,
and how the scanning function is constructed. Implemented in
`src-tauri/src/backup/{scan.rs,paths.rs}` and `src/pages/MigrationPlan.tsx`.

---

## 1. Problem

Walking the entire user profile (`C:\Users\<name>\`) drags in tens of GB of
junk: `AppData` caches, browser caches, launcher data, installer temp files.
Consequences:

- Backups take far longer than the file count suggests.
- USB space estimates are wrong; users buy bigger sticks than needed.
- Non-technical users cannot curate this themselves — and must never have to.
- OS/program files can never transfer usefully, yet they dominate the walk.

Ferry never walks `C:\Windows`, but AppData alone causes the same pain.

## 2. Goals / non-goals

**Goals**

1. Ask source OS (detected) → target OS (chosen installer) once.
2. Offer only folders that matter for that move, with measured sizes.
3. Walk **only** the selected folders; everything else is never even opened.
4. Refuse, loudly, any selected path outside the user profile.
5. Keep every existing safety property (checksums, manifest, verify-before-erase).

**Non-goals (for now)**

- Game-save detection inside AppData (warned about, not solved).
- Skipping files already in Dropbox/OneDrive/Google Drive.
- Linux/macOS as backup hosts (matrix reserves the rows).

## 3. Concepts

| Term | Meaning |
|---|---|
| Source family | `windows` / `linux` / `unknown`, from OS caption |
| Target family | Mapped from the chosen `source_id` (`windows-*` → windows, `ubuntu-*` → linux) |
| Profile folder | One top-level directory of the home folder, measured (bytes + file count) |
| Recommended set | Desktop, Documents, Downloads, Pictures, Music, Videos (personal, portable) |
| Roots | Absolute folder paths the user ticked; the scan's entire universe |

## 4. OS matrix

| Source → Target | Recommended | Warnings shown |
|---|---|---|
| Windows → Windows | Big six | Same-family note; apps reinstall via picker afterwards |
| Windows → Linux | Big six | Wi-Fi can't transfer; AppData/game saves stay behind; exFAT reads fine on Ubuntu |
| * → unknown | Big six | Target unrecognised; choose carefully |
| Linux → * | Big six | Row reserved until Linux hosts are supported |

Browser data and Wi-Fi export stay **separate passes** (they need special
handling, not folder walking) and are covered by the same encryption.

## 5. Backend contract

```
migration_profile(target_id: String) -> MigrationProfile
  { source_os, target_family, folders[{name, path, size_bytes, file_count, recommended}], warnings[] }

scan_user_files(extra_excludes: Vec<String>, roots: Vec<String>) -> ScanResult
  roots = []  → whole home (back-compat fallback)
  roots = [..] → each must canonicalize AND sit inside the canonical home,
                 else the whole scan fails closed
```

Safety invariants (all enforced backend-side, never trusted from the UI):

1. Every root is canonicalized, then `ensure_inside(home_canon, root)`.
2. Walking uses canonical paths only, so `normalise_path` prefix-stripping agrees
   (verbatim `\\?\` paths would otherwise poison every relative path).
3. `follow_links(false)` — symlinks/junctions are never followed.
4. Exclude rules from `scan.rs` still apply inside selected roots.

## 6. Scanning algorithm

**Pass 1 — measure** (`list_profile_folders`): one `read_dir` of home; one
recursive walk per top-level directory counting bytes + files. No hashing, no
copying. Sorted biggest-first so the worst offenders are visible.

**Pass 2 — scoped walk** (`scan_roots`): walk each selected root only; apply
default + extra excludes; emit `{source, relative, size_bytes}` per file with
`relative` rooted at the profile (e.g. `Users/John/Documents/...`).

**Pass 3+ (unchanged):** copy → browser/Wi-Fi/inventory → checksum manifest →
encrypt → verify-before-delete. The manifest records `original_path` for every
file, so restore needs no knowledge of the plan.

## 7. Folder taxonomy

- **Never walked:** `C:\Windows`, `Program Files`, AppData caches/temps (default
  exclude list), anything outside the selected roots.
- **Recommended personal:** Desktop, Documents, Downloads, Pictures, Music, Videos.
- **Available but unticked:** everything else top-level (including AppData, shown
  with its size so users see what they're skipping).
- **Separate passes:** browser profiles, Wi-Fi XML, registry/driver inventory.

## 8. Performance notes

- Measure pass costs one full walk of the profile (AppData included, to show its
  size). Acceptable: it replaces, not adds to, the old full scan.
- For very large profiles, progress display during measuring is a future
  improvement (currently a spinner on the Plan page).
- Hashing still happens once per file at verify time — unchanged.

## 9. Testing

- Unit (`cargo test`): family mapping, measure + recommend, scoped-walk
  subsetting, outside-profile rejection, Linux Wi-Fi warning presence.
- Manual (`test-plan.md` B3b): source detection correct; sizes sane;
  recommended pre-ticked; empty selection blocks Continue; Linux target shows
  the Wi-Fi warning.
- Adversarial: pass `C:\Windows` / `..` / non-existent roots → scan fails
  closed with a plain-language error, nothing copied.

## 10. Open questions

1. Should Downloads be recommended by default? (Often GBs of installers.)
2. Game-save allowlist for AppData subpaths (Steam `userdata`, etc.)?
3. Detect OneDrive/Dropbox placeholders to avoid double-storing cloud files?
4. Per-file-type caps (e.g. skip `*.tmp`, `node_modules`) as advanced options?
5. Multi-user machines: plan per Windows account or whole machine?

## 11. Implementation status

- [x] `backup/paths.rs`: measure, matrix, warnings (+ 4 unit tests)
- [x] `backup/scan.rs`: `scan_roots` with containment (+ 2 unit tests)
- [x] `migration_profile` command registered; `scan_user_files` takes `roots`
- [x] `MigrationPlan.tsx` between OSSelect and ExcludeReview
- [ ] Measure-pass progress events for huge profiles
- [ ] Game-save / cloud-placeholder awareness (§10)
