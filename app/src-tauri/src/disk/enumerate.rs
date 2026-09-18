/// Enumerate removable USB drives visible to Windows.
/// Returns only drives marked as removable — system/fixed drives are
/// intentionally excluded at the API level, not just in the UI.
use crate::types::DriveInfo;
use anyhow::Result;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};
use windows::Win32::System::WindowsProgramming::DRIVE_REMOVABLE;
use windows::core::PCWSTR;

/// Tauri command: returns list of removable drives with model, total, and free bytes.
#[tauri::command]
pub async fn list_removable_drives() -> Result<Vec<DriveInfo>, String> {
    enumerate_drives().map_err(|e| e.to_string())
}

fn enumerate_drives() -> Result<Vec<DriveInfo>> {
    let mut drives = Vec::new();

    // Check every possible drive letter A–Z.
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let drive_w: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0)).collect();

        // Safety: drive_w is a valid null-terminated wide string.
        let drive_type = unsafe { GetDriveTypeW(PCWSTR(drive_w.as_ptr())) };

        if drive_type != DRIVE_REMOVABLE {
            continue;
        }

        let mut free_bytes: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free: u64 = 0;

        let ok = unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(drive_w.as_ptr()),
                Some(&mut free_bytes),
                Some(&mut total_bytes),
                Some(&mut total_free),
            )
        };

        if ok.is_err() {
            continue;
        }

        let model = get_drive_model(&drive).unwrap_or_else(|_| "USB Drive".to_string());

        drives.push(DriveInfo {
            drive_letter: drive,
            model,
            total_bytes,
            free_bytes,
            is_removable: true,
        });
    }

    Ok(drives)
}

/// Use WMI via PowerShell to get the friendly model name for a drive letter.
/// Falls back to "USB Drive" if WMI is unavailable.
fn get_drive_model(drive_letter: &str) -> Result<String> {
    let letter = drive_letter.trim_end_matches('\\');
    let script = format!(
        "Get-WmiObject Win32_DiskDrive | Where-Object {{ $_.DeviceID -match (Get-Partition | Where-Object DriveLetter -eq '{}').DiskNumber }} | Select-Object -ExpandProperty Model",
        letter.trim_end_matches(':')
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()?;

    let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if model.is_empty() {
        Ok("USB Drive".to_string())
    } else {
        Ok(model)
    }
}

