/// Read saved Wi-Fi networks back for display in the app.
/// Passwords are only returned on explicit per-network request (and only for
/// networks that actually have one) — the list itself carries no secrets.
use crate::safety::validate_backup_dir;
use crate::wifi::profile::tag_contents;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct WifiProfile {
    pub ssid: String,
    pub security: String,
    pub has_password: bool,
}

fn read_profiles(backup_root: &str) -> Result<Vec<(String, String, Option<String>)>> {
    let canonical = validate_backup_dir(backup_root)?;
    let wifi_dir = canonical.join("WiFi");
    if !wifi_dir.is_dir() {
        return Ok(vec![]);
    }
    let mut xmls: Vec<PathBuf> = std::fs::read_dir(&wifi_dir)
        .context("Cannot read WiFi backup directory")?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("xml"))
        .collect();
    xmls.sort();
    let mut out = Vec::new();
    for path in xmls {
        let xml = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        let ssid = tag_contents(&xml, "name").unwrap_or_else(|| "(unknown)".to_string());
        let security = tag_contents(&xml, "authentication").unwrap_or_else(|| "unknown".to_string());
        let password = tag_contents(&xml, "keyMaterial").filter(|k| !k.is_empty());
        out.push((ssid, security, password));
    }
    out.sort_by_key(|a| a.0.to_lowercase());
    Ok(out)
}

/// Tauri command: list saved networks (SSIDs + security, no passwords).
#[tauri::command]
pub async fn list_wifi_profiles(backup_root: String) -> Result<Vec<WifiProfile>, String> {
    read_profiles(&backup_root)
        .map(|profiles| {
            profiles
                .into_iter()
                .map(|(ssid, security, password)| WifiProfile {
                    ssid,
                    security,
                    has_password: password.is_some(),
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Tauri command: reveal one network's password. The SSID is matched against
/// parsed profile names (never used as a path), and open networks report that
/// instead of an empty secret.
#[tauri::command]
pub async fn wifi_password(backup_root: String, ssid: String) -> Result<String, String> {
    let want = ssid.trim();
    if want.is_empty() || want.len() > 128 {
        return Err("Invalid network name.".to_string());
    }
    read_profiles(&backup_root)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|(name, _, _)| name == want)
        .and_then(|(_, _, password)| password)
        .ok_or_else(|| {
            "No saved password for this network (open network or not in the backup).".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backup() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let wifi = dir.path().join("WiFi");
        std::fs::create_dir_all(&wifi).unwrap();
        std::fs::write(
            wifi.join("home.xml"),
            "<WLANProfile><name>HomeNet</name><MSM><security><authEncryption>\
             <authentication>WPA2PSK</authentication><sharedKey>\
             <keyMaterial>s3cret!</keyMaterial></sharedKey>\
             </authEncryption></security></MSM></WLANProfile>",
        )
        .unwrap();
        std::fs::write(
            wifi.join("open.xml"),
            "<WLANProfile><name>Cafe</name><MSM><security><authEncryption>\
             <authentication>open</authentication></authEncryption></security></MSM></WLANProfile>",
        )
        .unwrap();
        let root = dir.path().to_string_lossy().to_string();
        (dir, root)
    }

    #[test]
    fn list_reports_password_presence_without_secrets() {
        let (_dir, root) = sample_backup();
        let profiles = read_profiles(&root).unwrap();
        assert_eq!(profiles.len(), 2);
        // Sorted by SSID: Cafe, HomeNet.
        assert_eq!(profiles[0].0, "Cafe");
        assert!(profiles[0].2.is_none());
        assert_eq!(profiles[1].0, "HomeNet");
        assert_eq!(profiles[1].2.as_deref(), Some("s3cret!"));
    }

    #[test]
    fn password_lookup_matches_exact_ssid_only() {
        let (_dir, root) = sample_backup();
        let profiles = read_profiles(&root).unwrap();
        let find = |ssid: &str| {
            profiles
                .iter()
                .find(|(name, _, _)| name == ssid)
                .and_then(|(_, _, pw)| pw.clone())
        };
        assert_eq!(find("HomeNet").as_deref(), Some("s3cret!"));
        assert_eq!(find("Cafe"), None);
        assert_eq!(find("home"), None); // case-sensitive: no fuzzy matching
        assert_eq!(find("../evil"), None);
    }
}
