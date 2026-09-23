/// Network-adapter preflight. This replaced the full driver inventory
/// (`driverquery` + `pnputil` over every device): Ferry can never install
/// drivers reliably, Windows Update resolves nearly all of them, and a
/// 200-row list is unactionable noise to a non-technical user.
///
/// The one driver question that matters for a migration is connectivity —
///
/// a machine whose network adapter has no driver on the new OS has no
/// recovery path at all. So this records exactly the network adapters:
/// what they are, whose driver they run, and which version, so the user
/// (or whoever helps them) can fetch the right driver on a second machine
/// if the new OS comes up offline.
use crate::safety::parse_csv_line;
use crate::types::DriverEntry;
use anyhow::{Context, Result};

/// Tauri command: list network adapters with driver details.
#[tauri::command]
pub async fn scan_network_adapters() -> Result<Vec<DriverEntry>, String> {
    scan().map_err(|e| e.to_string())
}

pub fn scan() -> Result<Vec<DriverEntry>> {
    let output = crate::proc::hidden("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-NetAdapter | Select-Object Name, InterfaceDescription, DriverProvider, DriverVersion, Status | ConvertTo-Csv -NoTypeInformation",
        ])
        .output()
        .context("Failed to query network adapters")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut adapters = Vec::new();
    let mut lines = stdout.lines();
    lines.next(); // header row

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        // Name, InterfaceDescription, DriverProvider, DriverVersion, Status.
        let cols = parse_csv_line(line);
        if cols.len() < 5 {
            continue;
        }
        let description = cols[1].trim().to_string();
        if description.is_empty() {
            continue;
        }
        let status = cols[4].trim().to_string();
        // A non-working adapter is the whole point of this list: flag it in
        // the name so it can't be skimmed past.
        let name = if status.eq_ignore_ascii_case("up") || status.is_empty() {
            description
        } else {
            format!("{} ({})", description, status)
        };
        adapters.push(DriverEntry {
            name,
            inf_name: None,
            provider: some_or_none(&cols[2]),
            version: some_or_none(&cols[3]),
            third_party: false,
        });
    }

    adapters.sort_by_key(|a| a.name.to_lowercase());
    Ok(adapters)
}

fn some_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, desc: &str, provider: &str, version: &str, status: &str) -> Vec<DriverEntry> {
        // Exercise the mapping through the real parser on one CSV line.
        let line = format!("\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"", name, desc, provider, version, status);
        let cols = parse_csv_line(&line);
        assert_eq!(cols.len(), 5);
        vec![DriverEntry {
            name: cols[1].trim().to_string(),
            inf_name: None,
            provider: some_or_none(&cols[2]),
            version: some_or_none(&cols[3]),
            third_party: false,
        }]
    }

    #[test]
    fn adapter_fields_map_cleanly() {
        let entries = row("Wi-Fi", "Intel Wireless-AC 9560", "Intel", "22.150.0.6", "Up");
        assert_eq!(entries[0].name, "Intel Wireless-AC 9560");
        assert_eq!(entries[0].provider.as_deref(), Some("Intel"));
        assert_eq!(entries[0].version.as_deref(), Some("22.150.0.6"));
    }

    #[test]
    fn empty_provider_becomes_none() {
        let entries = row("Ethernet", "Realtek PCIe GbE", "", "", "Up");
        assert!(entries[0].provider.is_none());
        assert!(entries[0].version.is_none());
    }
}
