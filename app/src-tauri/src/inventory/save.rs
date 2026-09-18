/// Persist and read the app/driver inventory alongside the file backup.
/// `save_inventory` writes `Backup/apps.json` + `Backup/drivers.json` before
/// verification so they are covered by the manifest and encryption.
/// `read_inventory` reads them back from a decrypted staging directory.
use crate::safety::{validate_backup_dir, validate_usb_root};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct InventoryFiles {
    pub apps: serde_json::Value,
    pub drivers: serde_json::Value,
}

/// Tauri command: validate and store inventory JSON in `<usb_root>/Backup/`.
#[tauri::command]
pub async fn save_inventory(
    usb_root: String,
    apps_json: String,
    drivers_json: String,
) -> Result<(), String> {
    run_save(PathBuf::from(usb_root), &apps_json, &drivers_json).map_err(|e| e.to_string())
}

fn run_save(usb_root: PathBuf, apps_json: &str, drivers_json: &str) -> Result<()> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let backup_dir = canonical_usb.join("Backup");
    if !backup_dir.is_dir() {
        anyhow::bail!("Backup/ folder not found; run file backup first");
    }
    let apps: serde_json::Value =
        serde_json::from_str(apps_json).context("apps inventory is not valid JSON")?;
    let drivers: serde_json::Value =
        serde_json::from_str(drivers_json).context("drivers inventory is not valid JSON")?;
    std::fs::write(
        backup_dir.join("apps.json"),
        serde_json::to_string_pretty(&apps)?,
    )
    .context("Cannot write Backup/apps.json")?;
    std::fs::write(
        backup_dir.join("drivers.json"),
        serde_json::to_string_pretty(&drivers)?,
    )
    .context("Cannot write Backup/drivers.json")?;
    Ok(())
}

/// Tauri command: read inventory JSON from a decrypted staging `Backup/` dir.
#[tauri::command]
pub async fn read_inventory(backup_dir: String) -> Result<InventoryFiles, String> {
    run_read(PathBuf::from(backup_dir)).map_err(|e| e.to_string())
}

fn run_read(backup_dir: PathBuf) -> Result<InventoryFiles> {
    let backup_str = backup_dir.to_string_lossy().to_string();
    let canonical = validate_backup_dir(&backup_str)?;
    let apps_raw = std::fs::read_to_string(canonical.join("apps.json"))
        .context("Backup/apps.json not found in this backup")?;
    let drivers_raw = std::fs::read_to_string(canonical.join("drivers.json"))
        .context("Backup/drivers.json not found in this backup")?;
    Ok(InventoryFiles {
        apps: serde_json::from_str(&apps_raw).context("Backup/apps.json is invalid")?,
        drivers: serde_json::from_str(&drivers_raw).context("Backup/drivers.json is invalid")?,
    })
}
