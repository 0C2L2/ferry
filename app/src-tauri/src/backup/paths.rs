/// Migration planning: what to back up, based on source → target OS.
///
/// Instead of blindly walking the whole profile (including multi-GB AppData
/// caches), the UI asks where the user is going and offers the folders that
/// actually matter for that move — with sizes, so the choice is informed.
use crate::backup::checksum::get_os_version;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

/// One top-level folder in the user's profile, with its measured size.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileFolder {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub recommended: bool,
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
    let folders = list_profile_folders(&home)?;
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
        folders.push(ProfileFolder {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes,
            file_count,
            recommended: is_recommended,
        });
    }
    folders.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
    Ok(folders)
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
