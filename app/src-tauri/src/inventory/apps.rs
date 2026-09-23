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

            // User apps only: skip patches, OS updates, runtimes that
            // reinstall automatically with their app, and anything Microsoft.
            let system_comp: String = subkey.get_value("SystemComponent").unwrap_or_else(|_| "0".to_string());
            let release_type: String = subkey.get_value("ReleaseType").unwrap_or_default();
            let parent_key: String = subkey.get_value("ParentKeyName").unwrap_or_default();
            if skip_reason(&name, &subkey_name, &system_comp, &release_type, &parent_key).is_some() {
                continue;
            }
            let publisher: String = subkey.get_value("Publisher").unwrap_or_default();
            if skip_by_publisher(&publisher).is_some() {
                continue;
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

/// Decide whether a registry uninstall entry is system noise rather than a
/// user-installed app. Returns the reason when skipping. Pure function so the
/// rules are unit-testable without touching a real registry.
fn skip_reason(
    name: &str,
    subkey_name: &str,
    system_comp: &str,
    release_type: &str,
    parent_key: &str,
) -> Option<&'static str> {
    if system_comp == "1" {
        return Some("system component");
    }
    if subkey_name.to_uppercase().starts_with("KB") {
        return Some("Windows update entry");
    }
    // Patches reference their parent product — never standalone apps.
    if !parent_key.trim().is_empty() {
        return Some("update patch for another product");
    }
    let release = release_type.to_lowercase();
    if release.contains("update") || release.contains("hotfix") || release.contains("service pack") {
        return Some("OS update");
    }
    let lower = name.to_lowercase();
    // Product decision: no Microsoft software in the list at all (the
    // publisher rule catches the rest; this covers entries without one).
    if lower.starts_with("microsoft ") || is_microsoft_brand(&lower) {
        return Some("Microsoft software");
    }
    for prefix in ["update for ", "security update for ", "hotfix for "] {
        if lower.starts_with(prefix) {
            return Some("OS update");
        }
    }
    if has_kb_number(&lower) {
        return Some("OS update");
    }
    // Redistributable runtimes reinstall automatically with their app.
    if lower.contains("redistributable") {
        return Some("redistributable runtime");
    }
    // Drivers come with the hardware or Windows Update, never a user's choice.
    if lower.contains(" driver") || lower.starts_with("driver ") {
        return Some("hardware driver");
    }
    // Parts of another product, installed automatically alongside it.
    for part in [
        "vs_", // Visual Studio internals (vs_CoreEditorFonts…)
        "microsoft visual studio installer",
        "windows sdk",
        "windows software development kit",
        "microsoft windows application compatibility",
    ] {
        if lower.starts_with(part) {
            return Some("component of another product");
        }
    }
    if lower.contains("add-in for") || lower.contains("addin for") {
        return Some("plugin installed by another app");
    }
    // Anti-cheat drivers are installed by the game itself.
    if lower.starts_with("riot vanguard")
        || lower.contains("easy anti-cheat")
        || lower.contains("easyanticheat")
        || lower.contains("battleye")
    {
        return Some("anti-cheat installed by a game");
    }
    // .NET runtimes likewise ("Microsoft .NET …", "Microsoft Windows Desktop
    // Runtime …", "dotnet-…"); SDKs are deliberate dev installs, keep those.
    if (lower.contains("microsoft .net")
        || lower.contains("windows desktop runtime")
        || lower.starts_with("dotnet "))
        && !lower.contains("sdk")
    {
        return Some(".NET runtime");
    }
    None
}

/// Publisher-based rules: things that ship with the device or with a browser,
/// never something the user chose to install.
fn skip_by_publisher(publisher: &str) -> Option<&'static str> {
    let p = publisher.to_lowercase();
    // Product decision: Microsoft and its subsidiaries are dropped entirely
    // (VS Code, Office, Teams, Edge, Xbox…), not only the Windows parts.
    if p.contains("microsoft") {
        return Some("Microsoft software");
    }
    // Chromium "install as app" shortcuts register an uninstall entry whose
    // Publisher is the browser's own registry path ("BraveSoftware\Brave-
    // Browser"). They return with browser sync; there is nothing to install.
    if p.contains('\\') {
        return Some("web-app shortcut from a browser");
    }
    // Hardware vendors' software ships with the device or its driver package,
    // and none of it transfers to another machine or OS.
    for vendor in [
        "advanced micro devices",
        "nvidia",
        "intel corporation",
        "realtek",
        "qualcomm",
        "synaptics",
    ] {
        if p.contains(vendor) {
            return Some("hardware vendor software");
        }
    }
    None
}

/// Microsoft-owned brands whose apps don't say "Microsoft" in the name or
/// publisher (the Store lists LinkedIn under an opaque `CN=<GUID>` ID).
pub(super) fn is_microsoft_brand(lower_name: &str) -> bool {
    ["linkedin", "skype", "xbox", "mojang", "github"]
        .iter()
        .any(|b| lower_name.contains(b))
}

/// `KB` followed by 6+ digits, e.g. "(KB5034441)".
fn has_kb_number(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if bytes[i] == b'k'
            && bytes[i + 1] == b'b'
            && bytes[i + 2..i + 8].iter().all(|b| b.is_ascii_digit())
        {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::skip_reason;

    fn kept(name: &str) -> bool {
        skip_reason(name, "{12345678-1234-1234-1234-123456789012}", "0", "", "").is_none()
    }

    #[test]
    fn real_user_apps_survive() {
        for app in ["Steam", "Slack", "Google Chrome", "Zoom Workplace"] {
            assert!(kept(app), "{app} must be kept");
        }
    }

    #[test]
    fn microsoft_software_is_dropped() {
        for name in ["Microsoft 365", "Microsoft Edge", "Microsoft OneDrive", "GitHub Desktop"] {
            assert!(!kept(name), "{name} is Microsoft");
        }
        for publisher in ["Microsoft Corporation", "Microsoft Corp."] {
            assert!(super::skip_by_publisher(publisher).is_some());
        }
    }

    #[test]
    fn non_user_entries_seen_on_a_real_machine_are_filtered() {
        // Every one of these passed the old filter on a real Windows 11 PC.
        for name in [
            "NVIDIA Graphics Driver 592.82",
            "vs_CoreEditorFonts",
            "Microsoft Visual Studio Installer",
            "Windows SDK AddOn",
            "Windows Software Development Kit - Windows 10.0.26100.7705",
            "Microsoft Windows Application Compatibility Fix Database",
            "Microsoft Teams Meeting Add-in for Microsoft Office",
            "Riot Vanguard",
        ] {
            assert!(!kept(name), "{name} is not a user app");
        }
        assert!(super::skip_by_publisher("BraveSoftware\\Brave-Browser").is_some());
        assert!(super::skip_by_publisher("Advanced Micro Devices, Inc.").is_some());
        // Real user apps from the same machine must survive both rule sets.
        for (name, publisher) in [
            ("Docker Desktop", "Docker Inc."),
            ("VALORANT", "Riot Games, Inc"),
            ("Telegram Desktop", "Telegram FZ-LLC"),
            ("Obsidian", "Obsidian"),
        ] {
            assert!(kept(name), "{name} must be kept");
            assert!(super::skip_by_publisher(publisher).is_none(), "{publisher} must be kept");
        }
    }

    #[test]
    fn system_noise_is_filtered() {
        // (name, subkey, system_comp, release_type, parent_key)
        let cases = [
            ("Anything", "KB5034441", "0", "", "", "KB subkey"),
            ("Security Update for Windows", "{GUID}", "0", "Security Update", "", "release type"),
            ("Hotfix for X", "{GUID}", "0", "", "", "hotfix prefix"),
            ("Helper", "{GUID}", "1", "", "", "system component"),
            ("Real App", "{GUID}", "0", "", "{PARENT-GUID}", "patch"),
            ("Microsoft Visual C++ 2015-2022 Redistributable (x64)", "{GUID}", "0", "", "", "redist"),
            ("Microsoft Windows Desktop Runtime 8.0", "{GUID}", "0", "", "", "runtime"),
            ("Update for Microsoft Windows (KB5034441)", "{GUID}", "0", "", "", "kb number"),
        ];
        for (name, subkey, comp, rel, parent, why) in cases {
            assert!(
                skip_reason(name, subkey, comp, rel, parent).is_some(),
                "{name} should be skipped ({why})"
            );
        }
    }
}

