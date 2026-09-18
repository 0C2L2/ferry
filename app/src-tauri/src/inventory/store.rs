/// Microsoft Store app inventory via `Get-AppxPackage`.
/// The registry Uninstall pass misses Store apps, so this is the separate pass
/// required by `company/app-reinstall-picker.md`. Store apps resolve through
/// the normal tier logic afterwards (winget often covers them).
use crate::inventory::drivers::parse_csv_line;
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

fn scan() -> Result<Vec<AppEntry>> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-AppxPackage | Select-Object Name, Version, @{Name='Publisher';Expression={$_.Publisher}} | ConvertTo-Csv -NoTypeInformation",
        ])
        .output()
        .context("Failed to run Get-AppxPackage")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();
    let mut lines = stdout.lines();
    lines.next(); // header row

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line(line);
        if cols.len() < 2 {
            continue;
        }
        let name = cols[0].trim().to_string();
        if name.is_empty() {
            continue;
        }
        // Skip framework/resource packages — not user-facing apps.
        let lower = name.to_lowercase();
        if lower.contains(".net.")
            || lower.contains("vclibs")
            || lower.contains("resources.")
            || lower.ends_with(".resources")
        {
            continue;
        }
        apps.push(AppEntry {
            name,
            version: cols.get(1).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            publisher: cols.get(2).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),
            url_info: None,
            tier: 3,
            winget_id: None,
        });
    }
    Ok(apps)
}
