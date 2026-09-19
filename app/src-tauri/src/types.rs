use serde::{Deserialize, Serialize};

/// Everything the frontend needs to display a backup manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub original_path: String,
    pub backup_path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

/// A file that was skipped during backup, with the reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedEntry {
    pub path: String,
    pub reason: String,
}

/// The full backup manifest written to manifest.json inside Backup.enc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub ferry_version: String,
    pub created_at: String,
    pub source_user: String,
    pub source_os: String,
    pub files: Vec<ManifestEntry>,
    pub skipped: Vec<SkippedEntry>,
}

/// A removable drive as presented to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub drive_letter: String,
    pub model: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub is_removable: bool,
}

/// The two partitions created on a USB drive by `prepare_usb`, identified by
/// their actual assigned drive letters (never assumed to match the original
/// pre-partition letter, which diskpart's auto-`assign` does not guarantee).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbLayout {
    pub boot_letter: String,
    pub data_letter: String,
}

/// An installed application entry from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub url_info: Option<String>,
    pub tier: u8,            // 1 | 2 | 3
    pub winget_id: Option<String>,
}

/// A driver entry from driverquery / pnputil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverEntry {
    pub name: String,
    pub inf_name: Option<String>,
    pub provider: Option<String>,
    pub version: Option<String>,
    pub third_party: bool,
}

/// Result of a restore operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSummary {
    pub restored: u64,
    pub skipped: u64,
    pub failed: Vec<String>,
    pub wifi_restored: u32,
    pub wifi_failed: u32,
}

