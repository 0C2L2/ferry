/// Enumerate installed drivers using driverquery and pnputil.
/// driverquery gives the full list; pnputil identifies OEM (third-party) drivers.
use crate::types::DriverEntry;
use anyhow::{Context, Result};
use std::collections::HashSet;

/// Tauri command: scan drivers and return a combined list.
#[tauri::command]
pub async fn scan_drivers() -> Result<Vec<DriverEntry>, String> {
    scan().map_err(|e| e.to_string())
}

pub fn scan() -> Result<Vec<DriverEntry>> {
    let mut drivers = parse_driverquery()?;
    let mut seen: HashSet<String> = drivers.iter().map(|d| d.name.to_lowercase()).collect();
    for oem in get_oem_drivers()? {
        let name = oem.original.clone();
        if seen.insert(name.to_lowercase()) {
            drivers.push(DriverEntry {
                name,
                inf_name: Some(oem.original),
                provider: oem.provider,
                version: oem.version,
                third_party: true,
            });
        }
    }
    drivers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(drivers)
}

struct OemDriver {
    original: String,
    provider: Option<String>,
    version: Option<String>,
}

/// Run `pnputil /enum-drivers` and parse OEM driver blocks. Only values that
/// pnputil actually reports are used; nothing is guessed from module names.
fn get_oem_drivers() -> Result<Vec<OemDriver>> {
    let output = std::process::Command::new("pnputil")
        .args(["/enum-drivers"])
        .output()
        .context("Failed to run pnputil")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut drivers = Vec::new();
    let mut original: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut version: Option<String> = None;

    // pnputil output is blank-line separated blocks such as:
    // Published Name / Original Name / Provider Name / Driver Version.
    let flush = |original: &mut Option<String>,
                     provider: &mut Option<String>,
                     version: &mut Option<String>,
                     drivers: &mut Vec<OemDriver>| {
        if let Some(name) = original.take() {
            let name = name.trim().to_string();
            if !name.is_empty() {
                drivers.push(OemDriver {
                    original: name,
                    provider: provider.take().filter(|v| !v.trim().is_empty()),
                    version: version.take().filter(|v| !v.trim().is_empty()),
                });
            }
        }
        provider.take();
        version.take();
    };
    for line in stdout.lines() {
        if line.trim().is_empty() {
            flush(&mut original, &mut provider, &mut version, &mut drivers);
            continue;
        }
        let mut parts = line.splitn(2, ':');
        let key = parts.next().unwrap_or("").trim().to_lowercase();
        let value = parts.next().unwrap_or("").trim().to_string();
        match key.as_str() {
            "original name" => original = Some(value),
            "provider name" => provider = Some(value),
            "driver version" => version = Some(value),
            _ => {}
        }
    }
    flush(&mut original, &mut provider, &mut version, &mut drivers);
    Ok(drivers)
}

/// Run `driverquery /v /fo csv` and parse the CSV output into DriverEntry structs.
/// driverquery does not report INF/provider/version here, so those fields are
/// left empty rather than guessed; OEM details come from pnputil blocks.
fn parse_driverquery() -> Result<Vec<DriverEntry>> {
    let output = std::process::Command::new("driverquery")
        .args(["/v", "/fo", "csv"])
        .output()
        .context("Failed to run driverquery")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut drivers = Vec::new();
    let mut lines = stdout.lines();

    // Skip header row.
    lines.next();

    for line in lines {
        // CSV columns: Module Name, Display Name, Description, Driver Type,
        // Start Mode, State, Status, ... Parsed quote-aware because display
        // names and descriptions can contain commas.
        let cols = parse_csv_line(line);
        if cols.len() < 4 {
            continue;
        }

        let name = cols[1].trim().to_string(); // Display Name
        if name.is_empty() {
            continue;
        }

        drivers.push(DriverEntry {
            name,
            inf_name: None,
            provider: None,
            version: None,
            third_party: false,
        });
    }

    Ok(drivers)
}

/// Minimal RFC-4180-style CSV line parser: honours double-quoted fields and
/// escaped `""` quotes so commas inside names don't shift columns.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cols = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                cols.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    cols.push(current);
    cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parser_handles_quoted_commas() {
        let cols = parse_csv_line(
            "\"srv\",\"Print, Spooler\",\"Manages, printing\",\"Kernel\",\"Auto\",\"Running\",\"OK\"",
        );
        assert_eq!(cols.len(), 7);
        assert_eq!(cols[1], "Print, Spooler");
        assert_eq!(cols[2], "Manages, printing");
    }

    #[test]
    fn csv_parser_handles_escaped_quotes() {
        let cols = parse_csv_line("\"a\",\"b \"\"quoted\"\" c\",\"d\"");
        assert_eq!(cols[1], "b \"quoted\" c");
    }
}

