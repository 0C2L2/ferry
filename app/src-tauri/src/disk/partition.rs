/// Partition a USB drive: small FAT32 boot partition + large exFAT data partition.
/// Uses diskpart via a script file — requires the process to be running as Administrator.
///
/// SAFETY INVARIANT: this function must only be called after backup verification
/// has completed and returned Ok. The caller (Tauri command) enforces this.
use crate::safety::{
    ensure_removable_drive_letter, validate_drive_letter, validate_usb_root,
};
use crate::AppState;
use anyhow::{bail, Context, Result};
use std::io::Write;
use tauri::State;

/// Tauri command: partition the given drive letter and format both partitions.
/// `drive_letter` must be a removable drive (e.g. "E:").
/// `usb_root` must be the verified backup location on that same drive.
/// Verification is consumed one-time from backend state; a frontend boolean is
/// not accepted because it cannot prove the erase order was followed.
#[tauri::command]
pub async fn prepare_usb(
    drive_letter: String,
    usb_root: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let letter = validate_drive_letter(&drive_letter).map_err(|e| e.to_string())?;
    ensure_removable_drive_letter(letter).map_err(|e| e.to_string())?;
    let (canonical_root, root_letter) =
        validate_usb_root(&usb_root).map_err(|e| e.to_string())?;
    if root_letter != letter {
        return Err("USB backup location is not on the selected drive.".to_string());
    }

    {
        let mut verified = state.verified_roots.lock().map_err(|_| {
            "Backend verification state is unavailable; cannot unlock erase.".to_string()
        })?;
        let key = canonical_root.to_string_lossy().to_string();
        if !verified.remove(&key) {
            return Err(
                "Backup has not been verified in this session. Cannot erase the drive."
                    .to_string(),
            );
        }
    }

    // Re-check removability immediately before resolving and wiping the disk.
    ensure_removable_drive_letter(letter).map_err(|e| e.to_string())?;
    partition_drive(letter).await.map_err(|e| e.to_string())
}

async fn partition_drive(letter: char) -> Result<()> {
    // Resolve the disk number from the validated drive letter using PowerShell.
    // Only a single A-Z letter reaches the shell, so no shell injection is possible.
    let disk_number =
        get_disk_number(letter).context("Could not determine disk number for drive letter")?;
    ensure_removable_drive_letter(letter)?;

    let script = format!(
        "select disk {disk}\n\
         clean\n\
         convert mbr\n\
         create partition primary size=32\n\
         format fs=fat32 quick label=\"FERRY_BOOT\"\n\
         assign\n\
         create partition primary\n\
         format fs=exfat quick label=\"FERRY_DATA\"\n\
         assign\n\
         exit\n",
        disk = disk_number
    );

    run_diskpart_script(&script).context("diskpart failed")?;
    Ok(())
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
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!("diskpart exited with error: {} {}", stdout.trim(), stderr.trim());
    }
    Ok(())
}
