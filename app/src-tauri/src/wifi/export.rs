/// Export all saved Wi-Fi profiles (including network keys) using netsh.
/// Profiles are saved as XML files in Backup/WiFi/ on the USB.
use crate::safety::validate_usb_root;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Tauri command: export Wi-Fi profiles to <usb_root>/Backup/WiFi/
#[tauri::command]
pub async fn export_wifi_profiles(usb_root: String) -> Result<u32, String> {
    run_export(PathBuf::from(usb_root)).map_err(|e| e.to_string())
}

fn run_export(usb_root: PathBuf) -> Result<u32> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let wifi_dir = canonical_usb.join("Backup").join("WiFi");
    std::fs::create_dir_all(&wifi_dir)
        .context("Cannot create Backup/WiFi/ directory")?;
    let before = xml_file_names(&wifi_dir);

    // netsh wlan export profile exports all profiles as separate XML files.
    // key=clear includes the plaintext network key in the XML.
    let output = std::process::Command::new("netsh")
        .args([
            "wlan",
            "export",
            "profile",
            &format!("folder={}", wifi_dir.to_string_lossy()),
            "key=clear",
        ])
        .output()
        .context("Failed to run netsh wlan export")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("netsh wlan export failed: {}", stderr.trim());
    }

    // Count newly written *.xml files instead of parsing netsh's human-readable
    // (and localized) stdout, so this works on non-English Windows too.
    let after = xml_file_names(&wifi_dir);
    Ok(after.difference(&before).count() as u32)
}

fn xml_file_names(dir: &std::path::Path) -> std::collections::HashSet<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|x| x.to_str()) == Some("xml")
                })
                .filter_map(|e| {
                    e.file_name().to_str().map(|s| s.to_lowercase())
                })
                .collect()
        })
        .unwrap_or_default()
}

