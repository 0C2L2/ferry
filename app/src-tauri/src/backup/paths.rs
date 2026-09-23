/// Migration planning: what to back up, based on source → target OS.
///
/// Instead of blindly walking the whole profile (including multi-GB AppData
/// caches), the UI asks where the user is going and offers the folders that
/// actually matter for that move — with sizes, so the choice is informed.
use crate::backup::checksum::get_os_version;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// One top-level folder offered for backup, with its measured size.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileFolder {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub recommended: bool,
    /// Which drive this came from ("C:", "D:"). Folders outside the user
    /// profile appear here too — someone with code in `C:\AI` would otherwise
    /// lose it silently, because nothing ever told them it wasn't included.
    pub drive: String,
    /// False for the profile's own folders, true for anything found elsewhere
    /// on disk. The UI groups on this, so "outside your user folder" is a
    /// visible choice rather than a surprise.
    pub outside_profile: bool,
}

/// Top-level directories belonging to Windows or to installed programs.
///
/// This is why "just back up everything that isn't the OS" doesn't work on its
/// own: an app install is neither the OS nor a document, and on Ubuntu it is
/// worthless. Skipping these leaves the folders a human might have put their
/// own files in, which is the set worth showing.
///
/// `Windows.old` is deliberately NOT listed — it holds the previous install's
/// user files, which is exactly what someone migrating wants back.
fn skipped_top_level() -> &'static [&'static str] {
    &[
        "windows",
        "program files",
        "program files (x86)",
        "programdata",
        "perflogs",
        "recovery",
        "$recycle.bin",
        "system volume information",
        "$winreagent",
        "msocache",
        "config.msi",
        "intel",
        "amd",
        "nvidia",
        "onedrivetemp",
        "$sysreset",
        "documents and settings", // legacy junction; loops back into Users
    ]
}

fn is_skipped(name: &str) -> bool {
    skipped_top_level().contains(&name.to_lowercase().as_str())
}

/// Gate for a backup root that sits outside the user profile.
///
/// Relaxing the old profile-only rule is what lets `C:\AI` be backed up, but
/// the renderer is still untrusted input for filesystem walks, so this fails
/// closed. A path is acceptable only when it is a real directory, on a fixed
/// drive, at least one level below the drive root, and its top-level folder
/// is not one of Windows' own.
///
/// Rejecting the drive root matters: `C:\` would pull in Windows, every
/// program install and the page file, which is neither what the user meant nor
/// something that fits on a USB.
pub fn ensure_selectable_outside_profile(canonical: &Path) -> Result<()> {
    use anyhow::bail;

    if !canonical.is_dir() {
        bail!("not a folder");
    }

    let mut components = canonical.components();
    let Some(std::path::Component::Prefix(prefix)) = components.next() else {
        bail!("not a path on a local drive");
    };
    // Skip the root separator that follows the drive prefix.
    components.next();

    let Some(top) = components.next() else {
        bail!("a whole drive cannot be backed up — pick folders inside it");
    };
    let top_name = top.as_os_str().to_string_lossy().to_string();
    if is_skipped(&top_name) {
        bail!("{top_name} belongs to Windows or to installed programs");
    }

    let letter = prefix
        .as_os_str()
        .to_string_lossy()
        .chars()
        .next()
        .context("could not read the drive letter")?;
    if !is_fixed_drive(letter) {
        bail!("only folders on this PC's own drives can be backed up");
    }
    Ok(())
}

/// The migration plan handed to the frontend.
#[derive(Debug, Serialize)]
pub struct MigrationProfile {
    pub source_os: String,
    pub target_family: String, // "windows" | "linux" | "unknown"
    pub folders: Vec<ProfileFolder>,
    pub warnings: Vec<String>,
}

/// Folders worth carrying for a personal migration. AppData, program files
/// and OS state are deliberately absent — they never transfer usefully.
fn recommended_names() -> Vec<&'static str> {
    vec![
        "Desktop",
        "Documents",
        "Downloads",
        "Pictures",
        "Music",
        "Videos",
    ]
}

/// Tauri command: detect the source OS, map the target, measure profile
/// folders, and return per-move warnings.
#[tauri::command]
pub async fn migration_profile(target_id: String) -> Result<MigrationProfile, String> {
    build_profile(&target_id).map_err(|e| e.to_string())
}

fn build_profile(target_id: &str) -> Result<MigrationProfile> {
    let home = super::scan::profile_home()?;
    let source_os = get_os_version();
    let target_family = target_family_of(target_id).to_string();
    let mut folders = list_profile_folders(&home)?;
    // Anything the user keeps outside their profile — a project in C:\AI, a
    // scratch folder on D:. Unticked, but visible, so it is never lost
    // silently the way it was before.
    folders.extend(list_other_drive_folders(&home));
    let warnings = warnings_for(&source_os, &target_family);
    Ok(MigrationProfile {
        source_os,
        target_family,
        folders,
        warnings,
    })
}

fn target_family_of(target_id: &str) -> &'static str {
    let id = target_id.to_lowercase();
    if id.starts_with("windows") {
        "windows"
    } else if id.starts_with("ubuntu") || id.starts_with("linux") {
        "linux"
    } else {
        "unknown"
    }
}

fn source_family_of(source_os: &str) -> &'static str {
    let lower = source_os.to_lowercase();
    if lower.contains("windows") {
        "windows"
    } else if lower.contains("linux") || lower.contains("ubuntu") {
        "linux"
    } else {
        "unknown"
    }
}

/// Measure every top-level directory in `home` (single walk each).
fn list_profile_folders(home: &Path) -> Result<Vec<ProfileFolder>> {
    let recommended = recommended_names();
    let mut folders = Vec::new();
    let entries = std::fs::read_dir(home)
        .with_context(|| format!("Cannot read profile directory {}", home.display()))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let (size_bytes, file_count) = measure_dir(&path);
        let is_recommended = recommended.iter().any(|r| r.eq_ignore_ascii_case(&name));
        let drive = drive_label(&path);
        folders.push(ProfileFolder {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes,
            file_count,
            recommended: is_recommended,
            drive,
            outside_profile: false,
        });
    }
    folders.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
    Ok(folders)
}

/// "C:" from "C:\Users\me\Documents".
fn drive_label(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

/// Top-level folders on every fixed drive that are not Windows, not program
/// installs, and not the user profile (already listed separately).
///
/// This is the fix for files kept outside `C:\Users` — a project in `C:\AI`, a
/// scratch folder on `D:`. Ferry used to refuse those outright, so they were
/// lost without anyone being told. Nothing here is ticked by default; the user
/// decides, with sizes in front of them.
fn list_other_drive_folders(home: &Path) -> Vec<ProfileFolder> {
    let mut folders = Vec::new();

    for letter in b'A'..=b'Z' {
        let root = crate::safety::drive_root(letter as char);
        let root_path = Path::new(&root);

        // Fixed disks only: skip the USB we are about to erase, optical
        // drives, and network shares that would take minutes to measure.
        if !is_fixed_drive(letter as char) || !root_path.is_dir() {
            continue;
        }

        let Ok(entries) = std::fs::read_dir(root_path) else {
            continue; // Unreadable root: skip rather than fail the whole plan.
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.is_empty() || is_skipped(&name) {
                continue;
            }
            // "Users" is covered by the profile listing; offering it whole
            // would drag in every other account on the machine.
            if name.eq_ignore_ascii_case("users") || path == home {
                continue;
            }

            let (size_bytes, file_count) = measure_dir(&path);
            if file_count == 0 {
                continue; // Empty folder: noise in the list.
            }
            folders.push(ProfileFolder {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes,
                file_count,
                recommended: false, // Never automatic — always the user's call.
                drive: drive_label(&path),
                outside_profile: true,
            });
        }
    }

    folders.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
    folders
}

/// Win32 `DRIVE_FIXED`.
const DRIVE_FIXED: u32 = 3;

fn is_fixed_drive(letter: char) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    let root = crate::safety::drive_root(letter);
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    // Safety: `wide` is a valid null-terminated wide string.
    unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) == DRIVE_FIXED }
}

fn measure_dir(dir: &Path) -> (u64, u64) {
    let mut size = 0u64;
    let mut count = 0u64;
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            count += 1;
        }
    }
    (size, count)
}

/// Per-move warnings shown before the user picks folders.
fn warnings_for(source_os: &str, target_family: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    match (source_family_of(source_os), target_family) {
        (_, "linux") => {
            warnings.push(
                "Wi-Fi profiles can't move to Linux — reconnect to networks manually there."
                    .to_string(),
            );
            warnings.push(
                "Windows app data is skipped: some game saves live in AppData and won't carry over."
                    .to_string(),
            );
            warnings.push(
                "Good news: Ubuntu reads this USB's exFAT partition directly, so your files open normally."
                    .to_string(),
            );
        }
        ("windows", "windows") => {
            warnings.push(
                "Same-family move: your folders carry over as-is; apps reinstall from the checklist afterwards."
                    .to_string(),
            );
        }
        (_, "unknown") => {
            warnings.push(
                "Target system not recognised — defaults applied. Pick folders carefully.".to_string(),
            );
        }
        _ => {}
    }
    warnings.push(
        "Program files and AppData caches are never copied — only the folders you tick.".to_string(),
    );
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_family_mapping() {
        assert_eq!(target_family_of("windows-11"), "windows");
        assert_eq!(target_family_of("windows-10"), "windows");
        assert_eq!(target_family_of("ubuntu-2404-lts"), "linux");
        assert_eq!(target_family_of("something-else"), "unknown");
    }

    #[test]
    fn source_family_mapping() {
        assert_eq!(source_family_of("Microsoft Windows 11 Pro"), "windows");
        assert_eq!(source_family_of("Ubuntu 24.04 LTS"), "linux");
        assert_eq!(source_family_of(""), "unknown");
    }

    #[test]
    fn profile_folders_measure_and_recommend() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Documents")).unwrap();
        std::fs::create_dir_all(dir.path().join("AppData")).unwrap();
        std::fs::write(dir.path().join("Documents").join("a.txt"), b"12345678").unwrap();
        std::fs::write(dir.path().join("AppData").join("cache.bin"), b"1234").unwrap();

        let folders = list_profile_folders(dir.path()).unwrap();
        assert_eq!(folders.len(), 2);
        let docs = folders.iter().find(|f| f.name == "Documents").unwrap();
        assert!(docs.recommended);
        assert_eq!(docs.size_bytes, 8);
        assert_eq!(docs.file_count, 1);
        let appdata = folders.iter().find(|f| f.name == "AppData").unwrap();
        assert!(!appdata.recommended);
        // Biggest first.
        assert!(folders[0].size_bytes >= folders[1].size_bytes);
    }

    #[test]
    fn linux_warnings_mention_wifi() {
        let warnings = warnings_for("Microsoft Windows 10 Home", "linux");
        assert!(warnings.iter().any(|w| w.contains("Wi-Fi")));
    }
}
