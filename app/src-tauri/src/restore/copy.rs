/// Restore files from a decrypted Backup/ directory to Restored/ on the new desktop.
/// Verifies checksums from manifest.json before copying anything.
use crate::types::RestoreSummary;
use crate::backup::checksum::hash_file;
use crate::safety::{ensure_inside, sanitize_relative_path, validate_backup_dir};
use crate::types::Manifest;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Tauri command: verify and copy files from decrypted Backup/ to Restored/ on desktop.
#[tauri::command]
pub async fn restore_files(
    app: AppHandle,
    backup_dir: String,    // Path to the decrypted Backup/ staging dir
) -> Result<RestoreSummary, String> {
    run_restore(app, PathBuf::from(backup_dir))
        .await
        .map_err(|e| e.to_string())
}

async fn run_restore(app: AppHandle, backup_dir: PathBuf) -> Result<RestoreSummary> {
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

        let _ = app.emit("restore:progress", serde_json::json!({
            "stage": "restore",
            "current": i as u64 + 1,
            "total": total,
            "current_item": entry.original_path,
        }));
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

fn get_desktop() -> Result<PathBuf> {
    let profile = std::env::var("USERPROFILE")
        .context("USERPROFILE not set")?;
    Ok(PathBuf::from(profile).join("Desktop"))
}

