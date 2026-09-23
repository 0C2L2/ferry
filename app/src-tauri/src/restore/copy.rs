/// Restore files from a decrypted Backup/ directory to Restored/ on the new desktop.
/// Verifies checksums from manifest.json before copying anything.
///
/// Shared by the Windows GUI and the Linux `ferry-restore` CLI. Verify-then-copy
/// is the core safety promise of a restore, so it lives here once rather than
/// being reimplemented per platform; callers supply their own progress
/// reporting through the `progress` callback.
use crate::hashing::hash_file;
use crate::safety::{ensure_inside, sanitize_relative_path, validate_backup_dir};
use crate::types::Manifest;
use crate::types::RestoreSummary;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Tauri command: verify and copy files from decrypted Backup/ to Restored/ on desktop.
#[cfg(windows)]
#[tauri::command]
pub async fn restore_files(
    app: tauri::AppHandle,
    backup_dir: String, // Path to the decrypted Backup/ staging dir
) -> Result<RestoreSummary, String> {
    use tauri::Emitter;
    restore_to_desktop(PathBuf::from(backup_dir), &|current, total, item| {
        let _ = app.emit(
            "restore:progress",
            serde_json::json!({
                "stage": "restore",
                "current": current,
                "total": total,
                "current_item": item,
            }),
        );
    })
    .map_err(|e| e.to_string())
}

/// Verify every file against the manifest, then copy it into `Restored/` on the
/// desktop. `progress` is called once per file as `(done, total, path)`.
pub fn restore_to_desktop(
    backup_dir: PathBuf,
    progress: &dyn Fn(u64, u64, &str),
) -> Result<RestoreSummary> {
    let backup_str = backup_dir.to_string_lossy().to_string();
    let canonical_backup = validate_backup_dir(&backup_str)?;
    // Read manifest.json from the decrypted backup.
    let manifest_path = canonical_backup.join("manifest.json");
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .context("manifest.json not found in backup")?
    ).context("Failed to parse manifest.json")?;

    // Destination: Restored/ on the new user's desktop.
    let desktop = get_desktop()?;
    let restored_root = desktop.join("Restored");
    std::fs::create_dir_all(&restored_root)?;

    let total = manifest.files.len() as u64;
    let mut restored = 0u64;
    let mut skipped  = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for (i, entry) in manifest.files.iter().enumerate() {
        let relative = match sanitize_relative_path(&entry.backup_path) {
            Ok(relative) => relative,
            Err(e) => {
                failed.push(format!("Unsafe backup path {}: {}", entry.original_path, e));
                skipped += 1;
                continue;
            }
        };
        let src = canonical_backup.join(&relative);
        let dst = restored_root.join(&relative);
        if ensure_inside(&canonical_backup, &src).is_err()
            || ensure_inside(&restored_root, &dst).is_err()
        {
            failed.push(format!("Unsafe backup path: {}", entry.original_path));
            skipped += 1;
            continue;
        }

        // Verify checksum before copying.
        match hash_file(&src) {
            Ok(hash) if hash == entry.sha256 => {}
            Ok(hash) => {
                failed.push(format!(
                    "Checksum mismatch: {} (expected {}, got {})",
                    entry.original_path, entry.sha256, hash
                ));
                skipped += 1;
                continue;
            }
            Err(e) => {
                failed.push(format!("Cannot read {}: {}", entry.original_path, e));
                skipped += 1;
                continue;
            }
        }

        if let Some(parent) = dst.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                failed.push(format!("Cannot create folder for {}: {}", entry.original_path, e));
                skipped += 1;
                continue;
            }
        }

        match std::fs::copy(&src, &dst) {
            Ok(_) => match hash_file(&dst) {
                Ok(dest_hash) if dest_hash == entry.sha256 => restored += 1,
                Ok(dest_hash) => {
                    failed.push(format!(
                        "Copy verification failed for {} (expected {}, got {})",
                        entry.original_path, entry.sha256, dest_hash
                    ));
                    skipped += 1;
                }
                Err(e) => {
                    failed.push(format!("Cannot verify {} after copy: {}", entry.original_path, e));
                    skipped += 1;
                }
            },
            Err(e) => {
                failed.push(format!("Copy failed for {}: {}", entry.original_path, e));
                skipped += 1;
            }
        }

        progress(i as u64 + 1, total, &entry.original_path);
    }

    // Wi-Fi import is handled separately (wifi::import Tauri command).
    // Its results are merged into the summary by the frontend.
    Ok(RestoreSummary {
        restored,
        skipped,
        failed,
        wifi_restored: 0, // filled in by frontend after wifi::import returns
        wifi_failed: 0,
    })
}

#[cfg(windows)]
pub fn get_desktop() -> Result<PathBuf> {
    let profile = std::env::var("USERPROFILE").context("USERPROFILE not set")?;
    Ok(PathBuf::from(profile).join("Desktop"))
}

/// Ubuntu localises the desktop folder — it is `~/Bureau` in French,
/// `~/Escritorio` in Spanish, and can be disabled entirely. Hardcoding
/// `~/Desktop` would silently create a second folder the file manager doesn't
/// show, so ask XDG first and only fall back when it has no answer.
#[cfg(not(windows))]
pub fn get_desktop() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;

    if let Ok(out) = std::process::Command::new("xdg-user-dir").arg("DESKTOP").output() {
        if out.status.success() {
            let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
            // xdg-user-dir echoes $HOME back when the desktop dir is unset.
            if path.is_dir() && path.as_os_str() != home.as_str() {
                return Ok(path);
            }
        }
    }

    let fallback = PathBuf::from(&home).join("Desktop");
    if fallback.is_dir() {
        return Ok(fallback);
    }
    // No desktop directory at all (server install, minimal DE): the home
    // directory is still somewhere the user can find their files.
    Ok(PathBuf::from(home))
}

