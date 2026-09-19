/// Scan the Windows registry for installed applications.
/// Reads from the three standard Uninstall paths used by Windows installers.
use crate::types::AppEntry;
use anyhow::Result;
// winreg::enums::* provides HKEY_LOCAL_MACHINE and HKEY_CURRENT_USER.
// winreg::RegKey is the key handle; HKEY is the raw handle type.
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use winreg::RegKey;

/// Registry uninstall paths and the hive they live under.
/// Array of (&str, isize) because HKEY_LOCAL_MACHINE / HKEY_CURRENT_USER are
/// isize constants (raw handle values) in winreg 0.52.
const UNINSTALL_PATHS: &[(&str, isize)] = &[
    (r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", HKEY_LOCAL_MACHINE),
    (r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall", HKEY_LOCAL_MACHINE),
    (r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall", HKEY_CURRENT_USER),
];

/// Tauri command: scan the registry and return a deduplicated list of installed apps.
#[tauri::command]
pub async fn scan_installed_apps() -> Result<Vec<AppEntry>, String> {
    scan().map_err(|e| e.to_string())
}

pub fn scan() -> Result<Vec<AppEntry>> {
    let mut apps: Vec<AppEntry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (path, hive) in UNINSTALL_PATHS {
        // RegKey::predef takes an isize raw handle.
        let root = RegKey::predef(*hive);
        let Ok(uninstall) = root.open_subkey_with_flags(path, KEY_READ) else {
            continue;
        };

        for subkey_name in uninstall.enum_keys().filter_map(|k| k.ok()) {
            let Ok(subkey) = uninstall.open_subkey_with_flags(&subkey_name, KEY_READ) else {
                continue;
            };

            let name: String = subkey.get_value("DisplayName").unwrap_or_default();
            if name.is_empty() || seen.contains(&name) {
                continue;
            }

            // Skip system components and updates — not meaningful for the user.
            let system_comp: String = subkey.get_value("SystemComponent").unwrap_or_else(|_| "0".to_string());
            if system_comp == "1" {
                continue;
            }
            // Skip Windows updates (GUIDs starting with KB).
            if subkey_name.starts_with('{') || subkey_name.to_uppercase().starts_with("KB") {
                // Keep GUIDs (they're real apps); skip "KB..." update entries.
                if subkey_name.to_uppercase().starts_with("KB") {
                    continue;
                }
            }

            seen.insert(name.clone());
            apps.push(AppEntry {
                name,
                version: subkey.get_value("DisplayVersion").ok(),
                publisher: subkey.get_value("Publisher").ok(),
                url_info: subkey.get_value("URLInfoAbout").ok(),
                tier: 3,         // Default; resolved to 1 or 2 by picker.rs
                winget_id: None, // Resolved by picker.rs
            });
        }
    }

    // Merge Microsoft Store apps (separate pass: registry misses them).
    for store_app in super::store::scan_store_apps_sync() {
        if !seen.contains(&store_app.name) {
            seen.insert(store_app.name.clone());
            apps.push(store_app);
        }
    }

    // Sort alphabetically for consistent display.
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}
