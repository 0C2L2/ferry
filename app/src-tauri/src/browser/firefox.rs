/// Back up Firefox browser data: places.sqlite (bookmarks + history),
/// logins.json (saved passwords) and key4.db (the key database logins.json
/// needs). Firefox stores profiles under
/// %APPDATA%\Mozilla\Firefox\Profiles\<profile-id>.default\
///
/// NOTE: if the user set a Firefox Primary Password, logins.json cannot be
/// read on the new system without it. Ferry copies the files for manual
/// import; the UI must not promise automatic password transfer.
use crate::safety::validate_usb_root;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Result of a Firefox backup pass: what was copied vs. skipped (with reason).
#[derive(Debug, Serialize)]
pub struct BrowserBackup {
    pub backed_up: Vec<String>,
    pub skipped: Vec<String>,
}

/// Tauri command: copy Firefox profile data to the USB backup.
#[tauri::command]
pub async fn backup_firefox_data(usb_root: String) -> Result<BrowserBackup, String> {
    run_backup(PathBuf::from(usb_root))
        .map_err(|e| e.to_string())
}

fn run_backup(usb_root: PathBuf) -> Result<BrowserBackup> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let app_data = std::env::var("APPDATA").context("APPDATA not set")?;
    let profiles_dir = PathBuf::from(app_data)
        .join("Mozilla")
        .join("Firefox")
        .join("Profiles");

    let mut backed_up: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    if !profiles_dir.exists() {
        return Ok(BrowserBackup { backed_up, skipped }); // Firefox not installed
    }

    let dest_base = canonical_usb.join("Backup").join("BrowserData").join("Firefox");
    std::fs::create_dir_all(&dest_base)?;

    for entry in std::fs::read_dir(&profiles_dir)
        .context("Cannot read Firefox Profiles directory")?
        .filter_map(|e| e.ok())
    {
        let profile_path = entry.path();
        if !profile_path.is_dir() {
            continue;
        }

        let profile_name = profile_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let dest = dest_base.join(&profile_name);
        std::fs::create_dir_all(&dest)?;

        for file in &["places.sqlite", "logins.json", "key4.db"] {
            let src = profile_path.join(file);
            if !src.exists() {
                continue;
            }
            let dst = dest.join(file);
            match std::fs::copy(&src, &dst)
                .with_context(|| format!("Failed to copy Firefox {} from {}", file, profile_name))
            {
                Ok(_) => backed_up.push(format!("Firefox/{}/{}", profile_name, file)),
                Err(e) => skipped.push(format!(
                    "Firefox/{}/{}: copy failed (close Firefox and retry): {}",
                    profile_name, file, e
                )),
            }
        }
    }

    Ok(BrowserBackup { backed_up, skipped })
}
