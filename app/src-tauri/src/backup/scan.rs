/// Scan C:\Users\<username>\ and return a list of files to back up,
/// applying the default exclude list and any user-specified additions.
use anyhow::Result;
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

/// Tauri command: walk the user's home directory and produce a file list + skip list.
/// `extra_excludes`: additional paths the user toggled off in the UI.
#[tauri::command]
pub async fn scan_user_files(
    extra_excludes: Vec<String>,
) -> Result<ScanResult, String> {
    scan(extra_excludes).map_err(|e| e.to_string())
}

pub fn scan(extra_excludes: Vec<String>) -> Result<ScanResult> {
    let home = get_user_home()?;
    let mut excludes = default_excludes();
    for e in extra_excludes {
        excludes.insert(e.to_lowercase());
    }

    let mut files = Vec::new();
    let mut total_bytes: u64 = 0;
    let mut skipped_reasons: Vec<(String, String)> = Vec::new();

    for entry in WalkDir::new(&home)
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
        let relative = normalise_path(&path, &home);

        total_bytes += size;
        files.push(FileToBackup { source: path, relative, size_bytes: size });
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

fn get_user_home() -> Result<PathBuf> {
    let profile = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:\\Users\\User".to_string());
    Ok(PathBuf::from(profile))
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
}

