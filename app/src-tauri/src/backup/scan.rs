/// Scan C:\Users\<username>\ and return a list of files to back up,
/// applying the default exclude list and any user-specified additions.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A single file ready to be backed up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileToBackup {
    pub source: PathBuf,
    /// Relative path used inside Backup/ (drive letter normalised).
    pub relative: String,
    pub size_bytes: u64,
}

/// Result returned to the frontend before backup starts.
#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub files: Vec<FileToBackup>,
    pub total_bytes: u64,
    pub skipped_count: usize,
    pub skipped_reasons: Vec<(String, String)>, // (path, reason)
}

/// Default folder/file patterns that are always excluded.
/// The user can remove items from this list in the UI.
fn default_excludes() -> HashSet<String> {
    [
        "AppData\\Local\\Temp",
        "AppData\\Local\\Microsoft\\Windows\\INetCache",
        "AppData\\Local\\Microsoft\\Windows\\Explorer",
        "AppData\\LocalLow\\Microsoft\\CryptnetUrlCache",
        "AppData\\Roaming\\Microsoft\\Windows\\Recent",
        ".DS_Store",
        "Thumbs.db",
        "desktop.ini",
    ]
    .iter()
    .map(|s| s.to_lowercase())
    .collect()
}

/// Tauri command: walk the selected profile folders and produce a file list.
/// `extra_excludes`: additional paths the user toggled off in the UI.
/// `roots`: absolute profile-folder paths to walk; empty means the whole home.
#[tauri::command]
pub async fn scan_user_files(
    extra_excludes: Vec<String>,
    roots: Vec<String>,
) -> Result<ScanResult, String> {
    scan(extra_excludes, roots).map_err(|e| e.to_string())
}

pub fn scan(extra_excludes: Vec<String>, roots: Vec<String>) -> Result<ScanResult> {
    let home = get_user_home()?;
    scan_roots(&home, extra_excludes, &roots)
}

pub fn scan_roots(
    home: &Path,
    extra_excludes: Vec<String>,
    roots: &[String],
) -> Result<ScanResult> {
    let mut excludes = default_excludes();
    for e in extra_excludes {
        excludes.insert(e.to_lowercase());
    }

    // Resolve and contain every requested root inside the profile.
    // Everything downstream uses canonical paths so prefix-stripping agrees.
    let home_canon = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let mut walk_roots: Vec<PathBuf> = Vec::new();
    if roots.is_empty() {
        walk_roots.push(home_canon.clone());
    } else {
        for root in roots {
            let canon = PathBuf::from(root)
                .canonicalize()
                .with_context(|| format!("Selected folder does not exist: {root}"))?;
            crate::safety::ensure_inside(&home_canon, &canon)
                .with_context(|| format!("Selected folder is outside your profile: {root}"))?;
            walk_roots.push(canon);
        }
    }

    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut skipped_reasons: Vec<(String, String)> = Vec::new();

    for root in &walk_roots {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path().to_path_buf();

            if path.is_dir() {
                continue;
            }

            // Check if any path component matches an exclude pattern.
            let path_lower = path.to_string_lossy().to_lowercase();
            if let Some(reason) = is_excluded(&path_lower, &excludes) {
                skipped_reasons.push((path.to_string_lossy().to_string(), reason));
                continue;
            }

            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let relative = normalise_path(&path, &home_canon);

            total_bytes += size;
            files.push(FileToBackup { source: path, relative, size_bytes: size });
        }
    }

    Ok(ScanResult {
        files,
        total_bytes,
        skipped_count: skipped_reasons.len(),
        skipped_reasons,
    })
}

/// Returns the reason string if the path matches an exclude pattern, else None.
/// Folder patterns match whole path segments; file patterns match file names.
/// This avoids substring false positives such as `my-thumbs.db-backup`.
fn is_excluded(path_lower: &str, excludes: &HashSet<String>) -> Option<String> {
    let normalized = path_lower.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').collect();
    let file_name = segments.last().copied().unwrap_or("");
    for pattern in excludes {
        let pattern = pattern.replace('\\', "/");
        if pattern.contains('/') {
            if normalized == pattern
                || normalized.starts_with(&format!("{pattern}/"))
                || normalized.contains(&format!("/{pattern}/"))
                || normalized.ends_with(&format!("/{pattern}"))
            {
                return Some(format!("excluded: {}", pattern));
            }
        } else if file_name == pattern
            || segments.iter().any(|segment| *segment == pattern)
        {
            return Some(format!("excluded: {}", pattern));
        }
    }
    None
}

/// Convert an absolute path to a relative path suitable for the Backup/ folder.
/// `C:\Users\John\Documents\taxes\` → `Users/John/Documents/taxes`
/// The drive prefix is removed; backslashes become forward slashes.
pub fn normalise_path(abs: &Path, home: &Path) -> String {
    // Re-join with home stripping for a clean relative path.
    abs.strip_prefix(home.parent().unwrap_or(home))
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Tauri command: index files already on the USB under `Backup/` (browser
/// data, Wi-Fi profiles, inventory JSON) so they join the checksum manifest.
/// Without this, those files would be encrypted but never verified or
/// restored. `source` points at the USB copy itself, so the source/backup
/// hash comparison trivially holds while still guarding later corruption.
#[tauri::command]
pub async fn list_usb_backup_files(usb_root: String) -> Result<Vec<FileToBackup>, String> {
    list_usb_files(&usb_root).map_err(|e| e.to_string())
}

fn list_usb_files(usb_root: &str) -> Result<Vec<FileToBackup>> {
    let (canonical_usb, _) = crate::safety::validate_usb_root(usb_root)?;
    let backup = canonical_usb.join("Backup");
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(&backup)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(&backup)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        // Reject anything that doesn't survive sanitization (shouldn't happen
        // for paths we created, but fail closed regardless).
        let safe = crate::safety::sanitize_relative_path(&rel)?;
        out.push(FileToBackup {
            source: path.to_path_buf(),
            relative: safe.to_string_lossy().replace('\\', "/"),
            size_bytes: entry.metadata().map(|m| m.len()).unwrap_or(0),
        });
    }
    out.sort_by_key(|f| f.relative.clone());
    Ok(out)
}

fn get_user_home() -> Result<PathBuf> {
    let profile = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users\\User".to_string());
    Ok(PathBuf::from(profile))
}

/// Profile home shared with the migration planner.
pub(crate) fn profile_home() -> Result<PathBuf> {
    get_user_home()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_strips_drive_prefix() {
        let home = PathBuf::from("C:\\Users\\John");
        let abs  = PathBuf::from("C:\\Users\\John\\Documents\\file.pdf");
        let rel  = normalise_path(&abs, &home);
        assert!(rel.contains("John/Documents/file.pdf"));
        assert!(!rel.contains(':'));
    }

    #[test]
    fn temp_folder_is_excluded() {
        let excludes = default_excludes();
        let path = "c:\\users\\john\\appdata\\local\\temp\\somefile.tmp".to_string();
        assert!(is_excluded(&path, &excludes).is_some());
    }

    #[test]
    fn scoped_roots_only_walk_selected_folders() {
        // NOTE: the fake home lives under target/, not the system temp dir,
        // because default excludes skip anything under AppData\Local\Temp.
        let home = test_home("scoped-roots");
        std::fs::create_dir_all(home.join("Documents")).unwrap();
        std::fs::create_dir_all(home.join("Pictures")).unwrap();
        std::fs::write(home.join("Documents").join("a.txt"), b"12345678").unwrap();
        std::fs::write(home.join("Pictures").join("b.txt"), b"1234").unwrap();

        let docs = home.join("Documents").to_string_lossy().to_string();
        let result = scan_roots(&home, vec![], &[docs]).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.total_bytes, 8);
        assert!(result.files[0].relative.contains("Documents/a.txt"));
        assert!(!result.files[0].relative.contains(':'));

        // Whole-home scan finds both files.
        let all = scan_roots(&home, vec![], &[]).unwrap();
        assert_eq!(all.files.len(), 2);

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn roots_outside_profile_are_rejected() {
        let home = test_home("roots-reject");
        let outside = test_home("roots-outside");
        let evil = outside.to_string_lossy().to_string();
        assert!(scan_roots(&home, vec![], &[evil]).is_err());
        std::fs::remove_dir_all(&home).ok();
        std::fs::remove_dir_all(&outside).ok();
    }

    /// Unique fake profile home under target/ (see note above).
    fn test_home(name: &str) -> PathBuf {
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("ferry-test-{}-{}", name, std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}

