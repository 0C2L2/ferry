/// Enumerate removable USB drives visible to Windows.
/// Returns only drives marked as removable — system/fixed drives are
/// intentionally excluded at the API level, not just in the UI.
use crate::types::{DriveInfo, PartitionInfo};
use anyhow::Result;
// DRIVE_REMOVABLE lives in Win32_Storage_FileSystem in windows-rs 0.58.
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW,
};
use windows::core::PCWSTR;

/// Win32 DRIVE_REMOVABLE (= 2). Not re-exported as a typed symbol in windows-rs 0.58.
const DRIVE_REMOVABLE: u32 = 2;

/// Tauri command: returns list of removable drives with model, total, and free bytes.
#[tauri::command]
pub async fn list_removable_drives() -> Result<Vec<DriveInfo>, String> {
    enumerate_drives().map_err(|e| e.to_string())
}

fn enumerate_drives() -> Result<Vec<DriveInfo>> {
    let mut drives = Vec::new();

    for letter in b'A'..=b'Z' {
        let drive = crate::safety::drive_root(letter as char);
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
        let (label, filesystem) = volume_info(&drive).unwrap_or_default();

        // Resolve which physical disk this letter sits on, and how big that
        // disk is, before grouping. Kept here rather than inside the grouping
        // so the grouping stays a pure function that can be tested.
        let letter = drive.trim_end_matches('\\').trim_end_matches(':');
        let disk = disk_info_for_letter(letter);

        drives.push(DriveInfo {
            drive_letter: drive.clone(),
            model,
            // Whole-disk capacity when known: that is what `clean` reclaims.
            total_bytes: disk.map(|(_, bytes)| bytes).unwrap_or(total_bytes),
            free_bytes,
            is_removable: true,
            disk_number: disk.map(|(number, _)| number),
            partition_count: 1,
            partitions: vec![PartitionInfo {
                letter: drive.trim_end_matches('\\').to_string(),
                label,
                filesystem,
                total_bytes,
                free_bytes,
            }],
        });
    }

    Ok(collapse_to_physical_disks(drives))
}

/// Present one row per physical stick, sized by the whole disk.
///
/// The loop above walks drive *letters*, so a USB that already has two
/// partitions (a Ventoy stick, or one Ferry prepared earlier) shows up as two
/// separate entries for a single device — and `GetDiskFreeSpaceExW` reports
/// each *partition's* size, not the disk's. A 32 GB Ventoy stick therefore
/// listed as "VTOYEFI, 33 MB" next to "Ventoy, 29 GB", and picking the first
/// got rejected as too small for the OS image even though erasing it frees the
/// whole 32 GB.
///
/// Partitioning wipes the entire disk regardless of which letter was picked,
/// so the honest unit to show the user is the disk. Anything that can't be
/// resolved to a disk number is kept as-is rather than dropped — never hide a
/// drive from the user because a lookup failed.
fn collapse_to_physical_disks(drives: Vec<DriveInfo>) -> Vec<DriveInfo> {
    let mut by_disk: Vec<DriveInfo> = Vec::new();

    for drive in drives {
        match drive
            .disk_number
            .and_then(|n| by_disk.iter_mut().find(|d| d.disk_number == Some(n)))
        {
            Some(existing) => {
                existing.partition_count += 1;
                // Free space only matters before the wipe; show the roomiest
                // partition so the figure isn't misleadingly tiny for a stick
                // that is mostly empty.
                existing.free_bytes = existing.free_bytes.max(drive.free_bytes);
                existing.partitions.extend(drive.partitions);
            }
            None => by_disk.push(drive),
        }
    }

    by_disk
}

/// Volume label + filesystem for a drive root (`E:\`), best-effort: unready
/// volumes yield empty strings rather than hiding the drive.
fn volume_info(drive_root: &str) -> Result<(String, String), ()> {
    let root_w: Vec<u16> = drive_root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut label_buf = [0u16; 261];
    let mut fs_buf = [0u16; 261];
    // Safety: buffers are valid and sized; sizes derive from the slices.
    unsafe {
        GetVolumeInformationW(
            PCWSTR(root_w.as_ptr()),
            Some(&mut label_buf),
            None,
            None,
            None,
            Some(&mut fs_buf),
        )
    }
    .map_err(|_| ())?;
    Ok((wide_to_string(&label_buf), wide_to_string(&fs_buf)))
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Returns `(disk number, whole-disk size in bytes)` for a drive letter.
fn disk_info_for_letter(letter: &str) -> Option<(u32, u64)> {
    // `letter` came from our own A-Z loop, so it is a single ASCII character.
    let script = format!(
        "try {{ \
            $n = (Get-Partition -DriveLetter '{letter}' -ErrorAction Stop).DiskNumber; \
            $d = Get-Disk -Number $n -ErrorAction Stop; \
            \"$n|$($d.Size)\" \
        }} catch {{ '' }}"
    );
    let output = crate::proc::hidden("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let (number, size) = stdout.trim().split_once('|')?;
    Some((number.trim().parse().ok()?, size.trim().parse().ok()?))
}

/// Use WMI via PowerShell to get the friendly model name for a drive letter.
/// Falls back to "USB Drive" if WMI is unavailable.
fn get_drive_model(drive_letter: &str) -> Result<String> {
    // drive_letter is already validated to be a single letter + ":\" — safe to embed.
    let letter = drive_letter
        .trim_end_matches('\\')
        .trim_end_matches(':');
    // Use Get-PhysicalDisk which is more reliable than Win32_DiskDrive + partition join.
    let script = format!(
        "try {{ \
            $dl = '{letter}'; \
            $diskNum = (Get-Partition -DriveLetter $dl -ErrorAction Stop).DiskNumber; \
            (Get-Disk -Number $diskNum -ErrorAction Stop).FriendlyName \
        }} catch {{ 'USB Drive' }}"
    );

    let output = crate::proc::hidden("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()?;

    let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if model.is_empty() {
        Ok("USB Drive".to_string())
    } else {
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(letter: &str, disk: Option<u32>, total: u64, free: u64) -> DriveInfo {
        DriveInfo {
            drive_letter: format!("{letter}:\\"),
            model: "Generic Flash Disk".to_string(),
            total_bytes: total,
            free_bytes: free,
            is_removable: true,
            disk_number: disk,
            partition_count: 1,
            partitions: vec![PartitionInfo {
                letter: format!("{letter}:"),
                label: String::new(),
                filesystem: "exFAT".to_string(),
                total_bytes: total,
                free_bytes: free,
            }],
        }
    }

    #[test]
    fn partitions_of_one_disk_collapse_into_one_row() {
        // The real case: a Ventoy stick listed as VTOYEFI (33 MB) plus Ventoy
        // (29 GB). Two rows for one device, and picking the small one got
        // rejected as too small for the OS image.
        let rows = collapse_to_physical_disks(vec![
            drive("D", Some(1), 31_676_037_120, 30_000_000_000),
            drive("E", Some(1), 33_275_904, 1_000_000),
        ]);
        assert_eq!(rows.len(), 1, "one physical stick must be one row");
        assert_eq!(rows[0].partition_count, 2);
        // Free space reported from the roomiest partition, not the 33 MB one.
        assert_eq!(rows[0].free_bytes, 30_000_000_000);
        // Both partitions ride along for the dropdown.
        let letters: Vec<&str> = rows[0].partitions.iter().map(|p| p.letter.as_str()).collect();
        assert_eq!(letters, vec!["D:", "E:"]);
    }

    #[test]
    fn separate_disks_stay_separate() {
        let rows = collapse_to_physical_disks(vec![
            drive("D", Some(1), 31_676_037_120, 30_000_000_000),
            drive("F", Some(2), 8_000_000_000, 8_000_000_000),
        ]);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn unresolvable_disks_are_listed_not_hidden() {
        // A failed disk lookup must never make a drive vanish from the picker:
        // the user would have no way to select the stick in front of them.
        let rows = collapse_to_physical_disks(vec![
            drive("D", None, 16_000_000_000, 16_000_000_000),
            drive("E", None, 8_000_000_000, 8_000_000_000),
        ]);
        assert_eq!(rows.len(), 2);
    }
}
