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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{} {}", stdout, stderr).to_lowercase();
        // Machines without a wireless interface (many desktops, VMs) can't
        // export anything — that's a skip, not a failure.
        if combined.contains("no wireless interface") {
            return Ok(0);
        }
        let detail = format!("{} {}", stdout.trim(), stderr.trim()).trim().to_string();
        anyhow::bail!(
            "netsh wlan export failed{}",
            if detail.is_empty() {
                ". Is Wi-Fi available on this machine?".to_string()
            } else {
                format!(": {}", detail)
            }
        );
    }

    // Count newly written *.xml files instead of parsing netsh's human-readable
    // (and localized) stdout, so this works on non-English Windows too.
    let after = xml_file_names(&wifi_dir);
    let added: Vec<String> = after.difference(&before).cloned().collect();

    // Human-readable password sheet: if automatic import ever fails (or the
    // new OS is Linux), the user can reconnect by hand from the restored copy.
    // It lives inside Backup/, so it gets the same encryption as everything.
    if !added.is_empty() {
        write_password_sheet(&wifi_dir)?;
    }
    Ok(added.len() as u32)
}

/// `WiFi-Passwords.txt`: SSID + security + key parsed from each exported XML.
fn write_password_sheet(wifi_dir: &std::path::Path) -> Result<()> {
    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut xmls: Vec<PathBuf> = std::fs::read_dir(wifi_dir)
        .context("Cannot read Backup/WiFi/ directory")?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("xml"))
        .collect();
    xmls.sort();
    for path in xmls {
        let xml = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read {}", path.display()))?;
        let ssid = tag_contents(&xml, "name").unwrap_or_else(|| "(unknown)".to_string());
        let security = tag_contents(&xml, "authentication").unwrap_or_else(|| "unknown".to_string());
        let password = tag_contents(&xml, "keyMaterial")
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| "(none — open network)".to_string());
        entries.push((ssid, security, password));
    }
    entries.sort_by_key(|a| a.0.to_lowercase());

    let mut sheet = String::from(
        "Ferry Wi-Fi passwords — stored inside your encrypted backup.\n\
         If automatic Wi-Fi restore doesn't work on the new system,\n\
         reconnect by hand using this list. Keep it safe.\n",
    );
    for (ssid, security, password) in &entries {
        sheet.push_str("\n----------------------------------------\n");
        sheet.push_str(&format!("Network:  {}\nSecurity: {}\nPassword: {}\n", ssid, security, password));
    }
    std::fs::write(wifi_dir.join("WiFi-Passwords.txt"), sheet)
        .context("Cannot write WiFi-Passwords.txt")?;
    Ok(())
}

fn tag_contents(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    Some(decode_entities(xml[start..start + end].trim()))
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
    <name>Caf&amp; Bar</name>
    <MSM><security><authEncryption>
        <authentication>WPA2PSK</authentication>
        <sharedKey><keyType>passPhrase</keyType>
        <keyMaterial>hunter2</keyMaterial></sharedKey>
    </authEncryption></security></MSM>
</WLANProfile>"#;

    #[test]
    fn profile_fields_parse() {
        assert_eq!(tag_contents(SAMPLE, "name").as_deref(), Some("Caf& Bar"));
        assert_eq!(tag_contents(SAMPLE, "authentication").as_deref(), Some("WPA2PSK"));
        assert_eq!(tag_contents(SAMPLE, "keyMaterial").as_deref(), Some("hunter2"));
        assert!(tag_contents(SAMPLE, "nope").is_none());
    }
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

