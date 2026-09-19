/// Auto-detect a Ferry backup (Backup.enc) on any connected removable drive.
use anyhow::Result;
use serde::Serialize;
use windows::Win32::Storage::FileSystem::GetDriveTypeW;
use windows::core::PCWSTR;

/// Win32 DRIVE_REMOVABLE (= 2). Not re-exported as a typed symbol in windows-rs 0.58.
const DRIVE_REMOVABLE: u32 = 2;

#[derive(Debug, Serialize)]
pub struct BackupLocation {
    pub drive_letter: String,
    pub enc_path: String,
    pub salt_path: String,
}

/// Tauri command: scan removable drives for Backup.enc + backup.salt.
/// Returns the first valid backup location found, or None.
#[tauri::command]
pub async fn find_backup_on_usb() -> Result<Option<BackupLocation>, String> {
    find().map_err(|e| e.to_string())
}

fn find() -> Result<Option<BackupLocation>> {
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let drive_w: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();

        // Safety: drive_w is a valid null-terminated wide string.
        let drive_type = unsafe { GetDriveTypeW(PCWSTR(drive_w.as_ptr())) };
        if drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let enc_path  = std::path::PathBuf::from(&drive).join("Backup.enc");
        let salt_path = std::path::PathBuf::from(&drive).join("backup.salt");

        if enc_path.exists() && salt_path.exists() {
            return Ok(Some(BackupLocation {
                drive_letter: drive,
                enc_path: enc_path.to_string_lossy().to_string(),
                salt_path: salt_path.to_string_lossy().to_string(),
            }));
        }
    }
    Ok(None)
}
