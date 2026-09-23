/// Re-import Wi-Fi profiles from Backup/WiFi/ on the USB.
/// Called at restore time — reimports all exported XML profiles.
use crate::safety::validate_backup_dir;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Tauri command: reimport Wi-Fi profiles from <backup_root>/WiFi/
/// Returns (success_count, failure_count).
#[tauri::command]
pub async fn import_wifi_profiles(backup_root: String) -> Result<(u32, u32), String> {
    run_import(PathBuf::from(backup_root)).map_err(|e| e.to_string())
}

fn run_import(backup_root: PathBuf) -> Result<(u32, u32)> {
    let backup_str = backup_root.to_string_lossy().to_string();
    let canonical_backup = validate_backup_dir(&backup_str)?;
    let wifi_dir = canonical_backup.join("WiFi");
    if !wifi_dir.exists() {
        return Ok((0, 0)); // No Wi-Fi profiles in this backup — silent skip.
    }

    let mut success = 0u32;
    let mut failed = 0u32;

    for entry in std::fs::read_dir(&wifi_dir)
        .context("Cannot read WiFi backup directory")?
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("xml") {
            continue;
        }
        // Ignore implausibly large profile files before handing them to netsh.
        if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 1024 * 1024 {
            failed += 1;
            continue;
        }

        // Relative filename from inside the folder: netsh truncates long
        // absolute paths (see export.rs) and then reports "not found".
        let Some(file_name) = path.file_name() else {
            continue;
        };
        let result = crate::proc::hidden("netsh")
            .current_dir(&wifi_dir)
            .args([
                "wlan",
                "add",
                "profile",
                &format!("filename={}", file_name.to_string_lossy()),
            ])
            .output();

        match result {
            Ok(output) if output.status.success() => success += 1,
            _ => failed += 1,
        }
    }

    Ok((success, failed))
}

