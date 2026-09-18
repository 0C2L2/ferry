/// Back up Chromium-family browser data (Chrome, Edge, Brave).
/// Copies Bookmarks (JSON) and Login Data (SQLite) from every detected profile
/// (Default, Profile N, Guest Profile). These are personal data files, not
/// app binaries — they fit Ferry's philosophy.
///
/// IMPORTANT DPAPI LIMITATION: on Windows, Chromium encrypts saved passwords
/// inside `Login Data` with DPAPI keys tied to the old user profile and
/// machine. After a clean OS reinstall those keys are gone, so the copied
/// `Login Data` is kept for reference/manual migration only — it will not
/// just "work" on the new system. The UI must tell users to rely on browser
/// sync (or a manual CSV export) for passwords, never promise otherwise.
use crate::safety::validate_usb_root;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Result of a Chromium backup pass: what was copied vs. skipped (with reason).
#[derive(Debug, Serialize)]
pub struct BrowserBackup {
    pub backed_up: Vec<String>,
    pub skipped: Vec<String>,
}

/// Tauri command: copy browser data for all detected Chromium browsers.
/// Dest: `<usb_root>/Backup/BrowserData/<BrowserName>/<ProfileDir>/`
#[tauri::command]
pub async fn backup_chromium_data(usb_root: String) -> Result<BrowserBackup, String> {
    run_backup(PathBuf::from(usb_root))
        .map_err(|e| e.to_string())
}

fn run_backup(usb_root: PathBuf) -> Result<BrowserBackup> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let local_app_data = std::env::var("LOCALAPPDATA")
        .context("LOCALAPPDATA not set")?;
    let local = PathBuf::from(local_app_data);

    // (browser name, "User Data" directory)
    let browsers = vec![
        ("Chrome",  local.join("Google").join("Chrome").join("User Data")),
        ("Edge",    local.join("Microsoft").join("Edge").join("User Data")),
        ("Brave",   local.join("BraveSoftware").join("Brave-Browser").join("User Data")),
    ];

    let mut backed_up: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for (name, user_data_dir) in &browsers {
        if !user_data_dir.is_dir() {
            continue;
        }

        for profile_dir in chromium_profiles(user_data_dir) {
            let profile_name = profile_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Default".to_string());
            let dest = canonical_usb
                .join("Backup")
                .join("BrowserData")
                .join(name)
                .join(&profile_name);
            std::fs::create_dir_all(&dest)
                .with_context(|| format!("Cannot create BrowserData/{}/{}", name, profile_name))?;

            for file in &["Bookmarks", "Login Data"] {
                let src = profile_dir.join(file);
                if !src.exists() {
                    continue;
                }
                let dst = dest.join(file);
                match std::fs::copy(&src, &dst) {
                    Ok(_) => backed_up.push(format!("{}/{}/{}", name, profile_name, file)),
                    Err(e) => skipped.push(format!(
                        "{}/{}/{}: copy failed (close the browser and retry): {}",
                        name, profile_name, file, e
                    )),
                }
            }
        }
    }

    Ok(BrowserBackup { backed_up, skipped })
}

/// Every profile directory inside a Chromium "User Data" folder:
/// `Default`, `Profile 1..N`, and `Guest Profile`.
fn chromium_profiles(user_data_dir: &PathBuf) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    let entries = match std::fs::read_dir(user_data_dir) {
        Ok(entries) => entries,
        Err(_) => return profiles,
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name == "Default"
            || dir_name == "Guest Profile"
            || dir_name.starts_with("Profile ")
        {
            profiles.push(path);
        }
    }
    profiles.sort();
    profiles
}
