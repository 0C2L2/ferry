/// Resolve each app to a tier using winget catalog lookup.
/// Tier 1: found in winget catalog (has a verified install command).
/// Tier 2: not in winget but has a URLInfoAbout field from the registry.
/// Tier 3: no source found — shown greyed out, no button.
use crate::safety::is_http_url;
use crate::types::AppEntry;
use anyhow::Result;
use tauri::{AppHandle, Emitter};

/// Tauri command: take a list of apps (from apps.rs) and resolve tiers.
#[tauri::command]
pub async fn resolve_app_tiers(
    app: AppHandle,
    mut apps: Vec<AppEntry>,
) -> Result<Vec<AppEntry>, String> {
    resolve(&app, &mut apps).await.map_err(|e| e.to_string())?;
    Ok(apps)
}

/// Tauri command: install one Tier-1 app via winget.
/// The package ID is strictly validated (catalog IDs only) and passed as a
/// single argv element — never through a shell.
#[tauri::command]
pub async fn install_app(winget_id: String) -> Result<String, String> {
    install(&winget_id).await.map_err(|e| e.to_string())
}

async fn install(winget_id: &str) -> Result<String> {
    if winget_id.is_empty()
        || winget_id.len() > 128
        || !winget_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        anyhow::bail!("Refusing to install: invalid winget package ID");
    }
    let output = tokio::process::Command::new("winget")
        .args([
            "install",
            "--id",
            winget_id,
            "--exact",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--disable-interactivity",
        ])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run winget: {}", e))?;
    if output.status.success() {
        Ok(format!("Installed {}", winget_id))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "winget install failed for {}: {} {}",
            winget_id,
            stdout.trim(),
            stderr.trim()
        )
    }
}

async fn resolve(app: &AppHandle, apps: &mut [AppEntry]) -> Result<()> {
    let curated = load_curated_table();
    let total = apps.len() as u64;
    for (i, app_entry) in apps.iter_mut().enumerate() {
        // Heartbeat for the UI: winget lookups are slow (seconds per app),
        // so report each one — otherwise the stage looks hung.
        let _ = app.emit("backup:progress", serde_json::json!({
            "stage": "inventory",
            "current": i as u64 + 1,
            "total": total,
            "current_item": app_entry.name,
        }));
        if let Some(winget_id) = winget_lookup(&app_entry.name).await {
            app_entry.tier = 1;
            app_entry.winget_id = Some(winget_id);
        } else if let Some(url) = curated_lookup(&curated, &app_entry.name) {
            // Human-verified vendor homepage from the bundled curated table.
            // Still shown as an unverified vendor-site link in the UI.
            app_entry.tier = 2;
            if !app_entry.url_info.as_deref().is_some_and(is_http_url) {
                app_entry.url_info = Some(url);
            }
        } else if app_entry.url_info.as_deref().is_some_and(is_http_url) {
            app_entry.tier = 2;
        } else {
            app_entry.tier = 3;
        }
    }
    Ok(())
}

/// Bundled curated table of well-known vendor homepages for common unpackaged
/// software (see company/app-reinstall-picker.md §Tier 2).
/// Entries must be human-verified official homepages — never search results,
/// never deep installer links that can rot or be hijacked.
#[derive(Debug, serde::Deserialize)]
struct CuratedApp {
    name: String,
    url: String,
}

fn load_curated_table() -> Vec<CuratedApp> {
    let raw = include_str!("../../../curated-apps.json");
    let table: Vec<CuratedApp> = serde_json::from_str(raw).unwrap_or_default();
    table
        .into_iter()
        .filter(|e| !e.name.trim().is_empty() && is_http_url(&e.url))
        .collect()
}

fn curated_lookup(table: &[CuratedApp], app_name: &str) -> Option<String> {
    let needle = app_name.trim().to_lowercase();
    table
        .iter()
        .find(|e| e.name.trim().to_lowercase() == needle)
        .map(|e| e.url.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_table_loads_and_matches_exactly() {
        let table = load_curated_table();
        assert!(!table.is_empty());
        assert!(curated_lookup(&table, "Steam").is_some());
        assert!(curated_lookup(&table, "  steam  ").is_some());
        assert!(curated_lookup(&table, "Steam Installer").is_none());
        assert!(curated_lookup(&table, "").is_none());
    }
}

/// Run `winget search --name "<name>" --exact --accept-source-agreements`
/// and return the package ID if exactly one match is found.
async fn winget_lookup(name: &str) -> Option<String> {
    let output = tokio::process::Command::new("winget")
        .args([
            "search",
            "--name", name,
            "--exact",
            "--accept-source-agreements",
            "--disable-interactivity",
        ])
        .output()
        .await
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // winget output table has ID in the second column.
    // Skip header lines until we see the separator "---", then require exactly
    // one result row so an ambiguous exact-name match fails closed.
    let mut past_header = false;
    let mut rows: Vec<String> = Vec::new();
    for line in stdout.lines() {
        if line.trim_start().starts_with("---") {
            past_header = true;
            continue;
        }
        if past_header && !line.trim().is_empty() {
            rows.push(line.to_string());
        }
    }
    if rows.len() != 1 {
        return None;
    }
    let cols: Vec<&str> = rows[0]
        .split_whitespace()
        .collect();
    if cols.len() >= 2 {
        Some(cols[1].to_string()) // Package ID
    } else {
        None
    }
}

