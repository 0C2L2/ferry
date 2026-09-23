/// SHA-256 checksum every file in Backup/, write manifest.json, and verify.
/// The erase step no longer waits on this — partitioning now happens before
/// backup even starts (see disk/partition.rs) — but verification still gates
/// UI progression: BackupProgress only advances to the next wizard step once
/// this returns Ok, so nothing downstream (cloud upload, OS download,
/// bootloader) runs against an unverified backup in the normal flow.
use crate::backup::copy::copy_file_chunked;
use crate::backup::scan::FileToBackup;
use crate::safety::{
    ensure_inside, ensure_source_under_home, sanitize_relative_path, validate_usb_root,
};
use crate::types::{Manifest, ManifestEntry, SkippedEntry};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Tauri command: hash every file in the Backup/ folder and write manifest.json.
/// Returns the serialised manifest on success.
#[tauri::command]
pub async fn verify_backup(
    app: AppHandle,
    files: Vec<FileToBackup>,
    usb_root: String,
    skipped: Vec<(String, String)>,
) -> Result<Manifest, String> {
    build_and_verify_manifest(app, files, PathBuf::from(usb_root), skipped)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command: read and parse manifest.json from a decrypted staging
/// `Backup/` dir. Used to surface the backed-up OS edition so the user
/// reinstalls a matching edition (Windows licenses don't cross editions).
#[tauri::command]
pub async fn read_manifest(backup_dir: String) -> Result<Manifest, String> {
    read_manifest_file(backup_dir).map_err(|e| e.to_string())
}

fn read_manifest_file(backup_dir: String) -> Result<Manifest> {
    use crate::safety::validate_backup_dir;
    let backup_str = backup_dir;
    let canonical = validate_backup_dir(&backup_str)?;
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(canonical.join("manifest.json"))
            .context("manifest.json not found in backup")?,
    )
    .context("Failed to parse manifest.json")?;
    Ok(manifest)
}

async fn build_and_verify_manifest(
    app: AppHandle,
    files: Vec<FileToBackup>,
    usb_root: PathBuf,
    skipped_raw: Vec<(String, String)>,
) -> Result<Manifest> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let backup_root = canonical_usb.join("Backup");
    let total = files.len() as u64;
    let mut entries: Vec<ManifestEntry> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    for (i, file) in files.iter().enumerate() {
        let relative = match sanitize_relative_path(&file.relative) {
            Ok(relative) => relative,
            Err(e) => {
                failures.push(format!("{}: {}", file.source.display(), e));
                continue;
            }
        };
        let source = match resolve_source(&file.source, &canonical_usb) {
            Ok(source) => source,
            Err(e) => {
                failures.push(format!("{}: {}", file.source.display(), e));
                continue;
            }
        };
        let backup_path = backup_root.join(&relative);
        if let Err(e) = ensure_inside(&canonical_usb, &backup_path) {
            failures.push(format!("{}: {}", file.source.display(), e));
            continue;
        }

        match verify_one(&source, &backup_path, file.size_bytes, &relative) {
            Ok(entry) => entries.push(entry),
            Err(failure) => failures.push(failure),
        }

        let _ = app.emit("backup:progress", serde_json::json!({
            "stage": "verify",
            "current": i as u64 + 1,
            "total": total,
            "current_item": file.source.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        }));
    }

    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(10).collect();
        let mut msg = format!(
            "Verification failed for {} file(s):\n{}",
            failures.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        if failures.len() > 10 {
            msg.push_str(&format!("\n…and {} more", failures.len() - 10));
        }
        bail!("{}", msg);
    }

    let source_user = std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string());
    let skipped = skipped_raw
        .into_iter()
        .map(|(path, reason)| SkippedEntry { path, reason })
        .collect();

    let manifest = Manifest {
        ferry_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now().to_rfc3339(),
        source_user,
        source_os: get_os_version(),
        files: entries,
        skipped,
    };

    // Write manifest.json into Backup/ (will be encrypted with the rest).
    let manifest_path = backup_root.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest)
        .context("Failed to serialise manifest")?;
    std::fs::write(&manifest_path, json)
        .context("Failed to write manifest.json")?;

    Ok(manifest)
}

/// Two kinds of entry reach verification, and they need different rules:
///
/// * Files copied from the user's profile — source lives under `%USERPROFILE%`
///   and is compared against its copy on the USB.
/// * Files the browser / Wi-Fi / inventory passes wrote straight to the USB
///   (`list_usb_backup_files` feeds these in) — the source *is* the backup
///   copy, and it was never under home to begin with.
///
/// Applying the under-home check to both kinds failed every USB-resident file,
/// and since any failure aborts the run, that killed the whole backup at the
/// Verify stage.
fn resolve_source(source: &std::path::Path, usb_root: &std::path::Path) -> Result<PathBuf> {
    if source.starts_with(usb_root) {
        // Produced by our own walk of the canonical USB root, and the
        // destination is still checked with `ensure_inside` below.
        Ok(source.to_path_buf())
    } else {
        ensure_source_under_home(source)
    }
}

/// Verify one file: both copies readable and identical.
/// Files that change mid-backup (browser databases, active downloads) get one
/// re-copy + re-check before being declared failures — transient writes heal,
/// only persistently-changing files block the backup.
fn verify_one(
    source: &std::path::Path,
    backup_path: &std::path::Path,
    size_bytes: u64,
    relative: &std::path::Path,
) -> Result<ManifestEntry, String> {
    if let Some(hash) = check_pair(source, backup_path) {
        return Ok(entry_for(source, relative, size_bytes, hash));
    }
    copy_file_chunked(source, backup_path)
        .map_err(|e| format!("Cannot re-copy {}: {}", source.display(), e))?;
    check_pair(source, backup_path)
        .map(|hash| entry_for(source, relative, size_bytes, hash))
        .ok_or_else(|| {
            format!(
                "{} changed during backup and could not be captured consistently. \
                 Close programs using it and run backup again.",
                source.display()
            )
        })
}

/// Hash both sides; Some(hash) only if readable and equal.
fn check_pair(source: &std::path::Path, backup_path: &std::path::Path) -> Option<String> {
    match (hash_file(source), hash_file(backup_path)) {
        (Ok(a), Ok(b)) if a == b => Some(a),
        _ => None,
    }
}

fn entry_for(
    source: &std::path::Path,
    relative: &std::path::Path,
    size_bytes: u64,
    hash: String,
) -> ManifestEntry {
    ManifestEntry {
        original_path: source.to_string_lossy().to_string(),
        backup_path: relative.to_string_lossy().replace('\\', "/"),
        size_bytes,
        sha256: hash,
    }
}

pub fn hash_file(path: &std::path::Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Cannot open for hashing: {:?}", path))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub(crate) fn get_os_version() -> String {
    crate::proc::hidden("powershell")
        .args(["-NoProfile", "-Command",
            "(Get-WmiObject Win32_OperatingSystem).Caption"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn usb_resident_files_skip_the_under_home_check() {
        // Regression: browser/Wi-Fi/inventory files live on the USB, never
        // under home. Requiring every source to be under home failed all of
        // them, and one failure aborts the entire backup.
        let usb = std::path::Path::new("E:\\");
        let on_usb = std::path::Path::new("E:\\Backup\\WiFi\\net.xml");
        assert_eq!(resolve_source(on_usb, usb).unwrap(), on_usb.to_path_buf());

        // Anything outside the USB still has to prove it came from home.
        let elsewhere = std::path::Path::new("C:\\Windows\\System32\\config\\SAM");
        assert!(resolve_source(elsewhere, usb).is_err());
    }

    #[test]
    fn hash_is_deterministic() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"ferry test content").unwrap();
        let path = f.path().to_path_buf();
        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex is 64 chars
    }
}

