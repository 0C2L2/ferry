/// OS source manifest — loaded from the bundled os-sources.json.
/// Data-driven: new OS entries ship as config updates without a full app rebuild.
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Downloaded via Microsoft Media Creation Tool API.
    MicrosoftMct,
    /// Direct URL with a published checksum.
    DirectUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsSource {
    pub id: String,
    pub label: String,
    pub source_type: SourceType,
    /// For DirectUrl: the ISO download URL.
    pub url: Option<String>,
    /// For DirectUrl: URL to a file containing the expected SHA-256 hash.
    pub checksum_url: Option<String>,
    /// Approximate download size in bytes (for UI display).
    pub approx_bytes: Option<u64>,
}

/// Tauri command: return the list of available OS sources from the bundled manifest.
#[tauri::command]
pub fn list_os_sources() -> Result<Vec<OsSource>, String> {
    load_sources().map_err(|e| e.to_string())
}

fn load_sources() -> Result<Vec<OsSource>> {
    let raw = include_str!("../../../os-sources.json");
    let sources: Vec<OsSource> = serde_json::from_str(raw)?;
    Ok(sources)
}

/// Resolve one bundled source by ID. URLs/checksums are never accepted from the
/// frontend, so a compromised renderer cannot redirect downloads or hashes.
pub(crate) fn resolve_source(source_id: &str) -> Result<OsSource> {
    let sources = load_sources()?;
    sources
        .into_iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown OS source: {}", source_id))
}

