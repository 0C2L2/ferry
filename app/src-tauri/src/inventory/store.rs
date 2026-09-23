/// Microsoft Store app inventory via `Get-AppxPackage`.
/// The registry Uninstall pass misses Store apps, so this is the separate pass
/// required by `company/app-reinstall-picker.md`. Store apps resolve through
/// the normal tier logic afterwards (winget often covers them).
use crate::safety::parse_csv_line;
use crate::types::AppEntry;
use anyhow::{Context, Result};

/// Tauri command: list installed Store apps. Never fails the whole inventory —
/// returns an empty list if the query is unavailable.
#[tauri::command]
pub async fn scan_store_apps() -> Result<Vec<AppEntry>, String> {
    Ok(scan_store_apps_sync())
}

/// Synchronous scan used to merge Store apps into the registry inventory.
/// Failures yield an empty list so one unavailable source can't break backup.
pub fn scan_store_apps_sync() -> Vec<AppEntry> {
    scan().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{parse_csv_line, parse_store_row};

    fn row(name: &str, sig: &str) -> Option<crate::types::AppEntry> {
        parse_store_row(&parse_csv_line(&format!(
            "\"{}\",\"1.0\",\"CN=X\",\"{}\"",
            name, sig
        )))
    }

    #[test]
    fn system_signed_packages_are_skipped() {
        assert!(row("Microsoft.Windows.ShellExperienceHost", "System").is_none());
        assert!(row("MicrosoftWindows.Client.CBS", "system").is_none());
    }

    #[test]
    fn store_signed_user_apps_are_kept() {
        assert!(row("Microsoft.WindowsStore", "Store").is_some());
        assert!(row("SpotifyAB.SpotifyMusic", "Store").is_some());
    }

    #[test]
    fn preinstalled_and_stub_packages_are_not_user_apps() {
        use super::came_with_windows;
        let shipped: std::collections::HashSet<String> =
            ["microsoft.bingnews".to_string()].into_iter().collect();
        let app = |name: &str, publisher: &str| crate::types::AppEntry {
            name: name.into(),
            version: None,
            publisher: Some(publisher.into()),
            url_info: None,
            tier: 3,
            winget_id: None,
        };
        let ms = "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond";
        let none = Default::default();
        // Shipped with the image (read from the registry list), even non-MS.
        let oem: std::collections::HashSet<String> = ["ad2f1837.omencommandcenter".to_string()].into_iter().collect();
        assert!(came_with_windows(&app("AD2F1837.OMENCommandCenter", "CN=HP"), &oem));
        // Codecs even when the registry list is empty.
        assert!(came_with_windows(&app("Some.HEVCVideoExtension", "CN=X"), &none));
        // Anything Microsoft, Store purchases included.
        assert!(came_with_windows(&app("Microsoft.BingNews", ms), &shipped));
        assert!(came_with_windows(&app("Microsoft.MinecraftUWP", ms), &none));
        assert!(came_with_windows(&app("Clipchamp.Clipchamp", "CN=Microsoft Corporation"), &none));
        assert!(came_with_windows(&app("AD2F1837.OMENCommandCenter", "CN=ED34"), &none));
        assert!(came_with_windows(&app("7EE7776C.LinkedInforWindows", "CN=DDCC"), &none));
        // Real user installs survive.
        assert!(!came_with_windows(&app("SpotifyAB.SpotifyMusic", "CN=Spotify"), &none));
        assert!(!came_with_windows(&app("com.tinyspeck.slackdesktop", "CN=Slack"), &none));
    }

    #[test]
    fn missing_signature_column_defaults_to_kept() {
        // Older output shape without SignatureKind must not nuke the list.
        let cols = parse_csv_line("\"Some.App\",\"2.0\",\"CN=Y\"");
        assert!(parse_store_row(&cols).is_some());
    }
}

fn scan() -> Result<Vec<AppEntry>> {
    let output = crate::proc::hidden("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-AppxPackage | Select-Object Name, Version, @{Name='Publisher';Expression={$_.Publisher}}, SignatureKind | ConvertTo-Csv -NoTypeInformation",
        ])
        .output()
        .context("Failed to run Get-AppxPackage")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let shipped = shipped_package_names();
    let mut apps = Vec::new();
    let mut lines = stdout.lines();
    lines.next(); // header row

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line(line);
        if let Some(app) = parse_store_row(&cols) {
            if !came_with_windows(&app, &shipped) {
                apps.push(app);
            }
        }
    }
    Ok(apps)
}

/// Package names Windows itself records as shipped with the machine: inbox
/// apps (Bing News, Solitaire, Paint, codecs…) and OEM preinstalls (HP OMEN,
/// AMD Radeon…). Key names are `<Name>_<version>_<arch>__<publisherHash>`, so
/// the name is everything before the first underscore. Empty when unreadable —
/// the pattern rules in `came_with_windows` still apply.
fn shipped_package_names() -> std::collections::HashSet<String> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    const BASE: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Appx\AppxAllUserStore";

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut names = std::collections::HashSet::new();
    for sub in ["InboxApplications", "Applications"] {
        let Ok(key) = hklm.open_subkey_with_flags(format!(r"{BASE}\{sub}"), KEY_READ) else {
            continue;
        };
        for full in key.enum_keys().filter_map(|k| k.ok()) {
            if let Some(name) = full.split('_').next() {
                names.insert(name.to_lowercase());
            }
        }
    }
    names
}

/// True for Store packages to leave out: preinstalled, or from Microsoft.
///
/// The previous rule deliberately kept every Store-signed inbox app ("cheap
/// rows the user can ignore"). On a real Windows 11 PC that was 71 rows of
/// Bing News, codecs and language packs burying the handful of apps the user
/// actually picked.
fn came_with_windows(
    app: &AppEntry,
    shipped: &std::collections::HashSet<String>,
) -> bool {
    let lower = app.name.to_lowercase();
    if shipped.contains(&lower) {
        return true;
    }
    // Backstop for when the registry list can't be read: codecs and language
    // packs are never a user's app choice.
    if lower.contains("videoextension")
        || lower.contains("imageextension")
        || lower.contains("webmediaextensions")
        || lower.contains("languageexperiencepack")
    {
        return true;
    }
    // PC makers' utilities, by their Store publisher-ID prefix (the
    // registry "shipped" list misses some OEM installs, e.g. HP OMEN).
    for oem in [
        "ad2f1837.",                  // HP
        "e046963f.",                  // Lenovo
        "dellinc.",                   // Dell
        "b9ecED6f.",                  // ASUS
        "acerincorporated.",          // Acer
        "appup.",                     // Intel
        "realteksemiconductorcorp.",  // Realtek
        "nvidiacorp.",                // NVIDIA
        "advancedmicrodevicesinc-2.", // AMD
    ] {
        if lower.starts_with(&oem.to_lowercase()) {
            return true;
        }
    }
    // Product decision: no Microsoft apps at all, Store purchases included.
    lower.starts_with("microsoft")
        || super::apps::is_microsoft_brand(&lower)
        || app
            .publisher
            .as_deref()
            .is_some_and(|p| p.to_lowercase().contains("microsoft"))
}

/// Map one CSV row to an entry. Framework/resource packages and OS-signed
/// (`System`) inbox components are not user apps. Everything else stays —
/// Store-signed inbox apps (Solitaire, Clipchamp) are cheap rows the user can
/// ignore, and several resolve to Tier 1 via winget anyway.
fn parse_store_row(cols: &[String]) -> Option<AppEntry> {
    if cols.len() < 2 {
        return None;
    }
    let name = cols[0].trim().to_string();
    if name.is_empty() {
        return None;
    }
    if cols.get(3).map(|s| s.trim().eq_ignore_ascii_case("system")).unwrap_or(false) {
        return None;
    }
    let lower = name.to_lowercase();
    if lower.contains(".net.")
        || lower.contains("vclibs")
        || lower.contains("resources.")
        || lower.ends_with(".resources")
    {
        return None;
    }
    Some(AppEntry {
        name,
        version: cols.get(1).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        publisher: cols.get(2).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
        url_info: None,
        tier: 3,
        winget_id: None,
    })
}
