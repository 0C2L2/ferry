/// Partition a USB drive: small FAT32 boot partition + large exFAT data partition.
/// Uses diskpart via a script file — requires the process to be running as Administrator.
///
/// SAFETY INVARIANT: this is the point of no return for whatever was previously
/// on the drive, so it runs immediately after drive selection — before backup,
/// download, or bootloader install ever write anything. Nothing downstream can
/// lose data to this step because nothing downstream has written anything yet.
/// The only backend-enforced gate is "the drive is actually removable"; the
/// explicit user confirmation is a UI-level checkbox shown right before this call.
use crate::safety::{drive_root, ensure_removable_drive_letter, validate_drive_letter};
use crate::types::UsbLayout;
use anyhow::{bail, Context, Result};
use std::io::Write;

/// Tauri command: partition the given drive letter and format both partitions.
/// `drive_letter` must be a removable drive (e.g. "E:"). Returns the actual
/// drive letters diskpart assigned to each new partition — never assume these
/// match the original `drive_letter`, which is destroyed by `clean`.
#[tauri::command]
pub async fn prepare_usb(drive_letter: String) -> Result<UsbLayout, String> {
    let letter = validate_drive_letter(&drive_letter).map_err(|e| e.to_string())?;
    ensure_removable_drive_letter(letter).map_err(|e| e.to_string())?;
    partition_drive(letter).await.map_err(|e| e.to_string())
}

async fn partition_drive(letter: char) -> Result<UsbLayout> {
    // Resolve the disk number from the validated drive letter using PowerShell.
    // Only a single A-Z letter reaches the shell, so no shell injection is possible.
    let disk_number =
        get_disk_number(letter).context("Could not determine disk number for drive letter")?;
    ensure_removable_drive_letter(letter)?;

    // Boot partition is 100 MB, not 32 MB: FAT32 needs at least 65,527
    // clusters (~33.5 MB at 512-byte sectors), so a 32 MB partition sits right
    // at/below the floor and `format fs=fat32` fails on it. Ventoy hits the
    // same wall and formats its ~33 MB VTOYEFI partition as FAT16 for exactly
    // this reason. 100 MB is the standard EFI System Partition size and is
    // negligible on any stick large enough to hold an OS image anyway.
    let script = format!(
        "select disk {disk}\n\
         clean\n\
         convert mbr\n\
         create partition primary size=100\n\
         format fs=fat32 quick label=\"FERRY_BOOT\"\n\
         assign\n\
         create partition primary\n\
         format fs=exfat quick label=\"FERRY_DATA\"\n\
         assign\n\
         exit\n",
        disk = disk_number
    );

    run_diskpart_script(&script).context("diskpart failed")?;

    // diskpart's `assign` picks the next free letter, which is not guaranteed
    // to be `letter` (or in any particular order) — resolve both partitions by
    // the labels we just gave them instead of assuming anything about letters.
    // Windows takes a moment to surface a freshly formatted volume, so retry
    // briefly rather than failing on a race we created ourselves.
    let boot_letter = get_letter_by_label_retrying("FERRY_BOOT")
        .context("Partitioned the drive but could not find the new FERRY_BOOT partition")?;
    let data_letter = get_letter_by_label_retrying("FERRY_DATA")
        .context("Partitioned the drive but could not find the new FERRY_DATA partition")?;

    // Spelled as full drive roots (`E:\`), not bare letters: these get passed
    // straight back into commands that canonicalize them as paths.
    Ok(UsbLayout {
        boot_letter: drive_root(boot_letter),
        data_letter: drive_root(data_letter),
    })
}

/// Windows doesn't always surface a newly formatted volume immediately, so
/// poll briefly before declaring it missing.
fn get_letter_by_label_retrying(label: &str) -> Result<char> {
    let mut last_err = None;
    for attempt in 0..10 {
        match get_letter_by_label(label) {
            Ok(letter) => return Ok(letter),
            Err(e) => last_err = Some(e),
        }
        if attempt < 9 {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Volume '{label}' never appeared")))
}

fn get_letter_by_label(label: &str) -> Result<char> {
    // `label` is always one of our own two hardcoded constants above, never
    // renderer input, so interpolating it into the PowerShell command is safe.
    let script = format!("(Get-Volume -FileSystemLabel '{label}').DriveLetter");
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .context("Failed to run PowerShell")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let mut chars = trimmed.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Ok(c.to_ascii_uppercase()),
        _ => bail!("Could not resolve a drive letter for volume label '{label}' (got: '{trimmed}')"),
    }
}

fn get_disk_number(letter: char) -> Result<u32> {
    let script = format!("(Get-Partition -DriveLetter '{letter}').DiskNumber");
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .context("Failed to run PowerShell")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let num = stdout.trim().parse::<u32>()
        .context("Could not parse disk number from PowerShell output")?;
    Ok(num)
}

fn run_diskpart_script(script: &str) -> Result<()> {
    // Write the script to a temp file and close the handle BEFORE launching diskpart.
    // On Windows, diskpart cannot read a file that another process (us) still has open.
    let script_path = {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "ferry-diskpart-{}.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        {
            // File handle is dropped at the end of this inner block.
            let mut f = std::fs::File::create(&path)
                .context("Could not create diskpart script file")?;
            f.write_all(script.as_bytes())
                .context("Could not write diskpart script")?;
            f.flush().context("Could not flush diskpart script")?;
            // f is dropped here — handle closed, lock released.
        }
        path
    };

    let result = std::process::Command::new("diskpart")
        .args(["/s", script_path.to_str().context("Invalid diskpart script path")?])
        .output()
        .context("Failed to run diskpart");

    // Always remove the temp script even if diskpart failed.
    let _ = std::fs::remove_file(&script_path);

    let output = result?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        bail!("diskpart exited with error: {} {}", stdout.trim(), stderr.trim());
    }

    // diskpart exits 0 even when individual commands inside the script fail —
    // it reports the failure on stdout and moves on. Without this check a
    // failed format looks like success here, and the real error only surfaces
    // later as a confusing "could not find the FERRY_BOOT partition".
    if let Some(problem) = find_diskpart_error(&stdout) {
        bail!("diskpart reported: {}", problem);
    }
    Ok(())
}

/// Scans diskpart's own output for a reported failure. Returns the offending
/// line so the user sees diskpart's actual words, not a guess at what broke.
fn find_diskpart_error(stdout: &str) -> Option<String> {
    const MARKERS: [&str; 7] = [
        "has encountered an error",
        "is not valid",
        "access is denied",
        "no disk selected",
        "failed",
        "unable to",
        "cannot",
    ];
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| {
            let lower = line.to_lowercase();
            MARKERS.iter().any(|m| lower.contains(m))
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::find_diskpart_error;

    #[test]
    fn diskpart_success_output_is_not_flagged() {
        let ok = "DiskPart succeeded in cleaning the disk.\n\
                  DiskPart succeeded in creating the specified partition.\n\
                  DiskPart successfully formatted the volume.\n\
                  DiskPart successfully assigned the drive letter or mount point.";
        assert!(find_diskpart_error(ok).is_none());
    }

    #[test]
    fn diskpart_failure_is_detected_despite_exit_zero() {
        let bad = "DiskPart succeeded in creating the specified partition.\n\
                   The format failed to complete successfully.\n";
        assert!(find_diskpart_error(bad).is_some());
    }

    #[test]
    fn access_denied_is_detected() {
        let denied = "DiskPart has encountered an error: Access is denied.";
        assert!(find_diskpart_error(denied).is_some());
    }
}
