/// Makes the USB bootable the way every other USB writer does it: extract the
/// ISO's entire contents onto the FAT32 partition. The ISO already ships its
/// own vendor-signed bootloader at `/EFI/boot/bootx64.efi` and its own
/// `grub.cfg`, so once the files are in place UEFI firmware boots it directly.
/// Ferry writes no boot configuration of its own and bundles no bootloader.
///
/// This replaced a GRUB-loopback scheme (keep the ISO as a single file, hand-
/// write a `grub.cfg` that loopback-mounts it). That exists to dodge FAT32's
/// 4 GB per-file limit — but measuring the real Ubuntu 24.04.2 ISO showed its
/// largest member is `casper/minimal.squashfs` at 1.69 GB, comfortably under
/// the limit. The limit never bound, so the complexity bought nothing: it
/// added a hand-written boot config, a dependency on GRUB's exFAT module
/// loading, and partition-addressing assumptions, all on the one code path
/// that cannot be verified without a reboot. Plain extraction has none of
/// that and is the most-travelled boot path in existence.
///
/// The ISO file is deleted from the data partition afterwards — it has served
/// its purpose and would otherwise waste several GB that the user's backup
/// needs.
///
/// HONESTY NOTE: not boot-tested in this session (no UEFI VM/hardware
/// available). Boot-test on a spare machine or QEMU+OVMF before relying on it.
use crate::safety::{ensure_removable_drive_letter, sanitize_filename, validate_drive_letter};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// Tauri command: extract the OS image that `download_os_image` placed on the
/// data partition onto the bootable FAT32 partition.
#[tauri::command]
pub async fn write_bootloader(
    app: AppHandle,
    boot_letter: String,
    data_letter: String,
    iso_filename: String,
) -> Result<(), String> {
    install_bootloader(&app, &boot_letter, &data_letter, &iso_filename)
        .await
        .map_err(|e| e.to_string())
}

async fn install_bootloader(
    app: &AppHandle,
    boot_letter: &str,
    data_letter: &str,
    iso_filename: &str,
) -> Result<()> {
    let boot = validate_drive_letter(boot_letter)?;
    let data = validate_drive_letter(data_letter)?;
    ensure_removable_drive_letter(boot)?;
    ensure_removable_drive_letter(data)?;
    let filename = sanitize_filename(iso_filename)?;

    let data_root = PathBuf::from(format!("{data}:\\"))
        .canonicalize()
        .context("Data partition is not available")?;
    let iso_path = data_root.join(&filename);
    if !iso_path.exists() {
        bail!(
            "OS image '{}' was not found on the data partition — the download may not have completed",
            filename
        );
    }

    let boot_root = PathBuf::from(format!("{boot}:\\"))
        .canonicalize()
        .context("Boot partition is not available")?;

    let mounted_letter = mount_iso(&iso_path)?;
    let copy_result = extract_iso(app, mounted_letter, &boot_root);
    // Always dismount, even if copying failed, so we never leak a drive letter.
    let dismount_result = dismount_iso(&iso_path);
    copy_result?;
    dismount_result?;

    // The ISO has been unpacked; keeping it would waste GBs the backup needs.
    std::fs::remove_file(&iso_path)
        .with_context(|| format!("Could not remove {} after extracting it", iso_path.display()))?;
    Ok(())
}

/// Copies every file out of the mounted ISO onto the boot partition, emitting
/// progress as it goes — this moves gigabytes and a silent UI invites the user
/// to pull the drive mid-write.
fn extract_iso(app: &AppHandle, mounted_letter: char, boot_root: &Path) -> Result<()> {
    let iso_root = PathBuf::from(format!("{mounted_letter}:\\"));
    if !iso_root.join("EFI").is_dir() {
        bail!("This OS image has no EFI directory — Ferry cannot make it bootable");
    }

    let files: Vec<_> = walkdir::WalkDir::new(&iso_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    let total = files.len() as u64;

    // FAT32 cannot hold a file of 4 GiB or more. Ubuntu never hits this, but
    // a Windows ISO can (install.wim). Fail BEFORE copying anything, with the
    // reason spelled out, instead of dying halfway through a multi-GB copy.
    for entry in files.iter() {
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        fits_fat32(&entry.path().display().to_string(), size)?;
    }

    for (i, entry) in files.iter().enumerate() {
        let rel = entry
            .path()
            .strip_prefix(&iso_root)
            .context("ISO entry escaped the image root")?;
        let dest = boot_root.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Could not create {}", parent.display()))?;
        }
        // Content check, not just byte counts: hash the source WHILE copying
        // (one pass) and the written file after, then compare. A bit-rotted
        // or short-written file of the right length would pass a size check
        // and only fail at boot time on another machine — the exact failure
        // this step exists to prevent.
        let src_hash = hash_while_copying(entry.path(), &dest)
            .with_context(|| format!("Could not copy {} to the boot partition", rel.display()))?;
        let dst_hash = crate::hashing::hash_file(&dest)
            .with_context(|| format!("Could not re-read {}", dest.display()))?;
        if src_hash != dst_hash {
            bail!(
                "Copy verification failed for {} (content mismatch after write)",
                rel.display()
            );
        }

        let _ = app.emit(
            "bootloader:progress",
            serde_json::json!({
                "stage": "extract",
                "current": i as u64 + 1,
                "total": total,
                "current_item": rel.to_string_lossy(),
            }),
        );
    }
    Ok(())
}


/// Copies one file while hashing what was read, returning the source hash.
/// The caller hashes the destination separately and compares: equal hashes
/// prove the write is bit-identical, regardless of lengths matching.
fn hash_while_copying(src: &Path, dst: &Path) -> Result<String> {
    let mut src_file =
        std::fs::File::open(src).with_context(|| format!("Cannot open {}", src.display()))?;
    let out_file =
        std::fs::File::create(dst).with_context(|| format!("Cannot create {}", dst.display()))?;
    let mut writer = std::io::BufWriter::new(out_file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = src_file
            .read(&mut buf)
            .with_context(|| format!("Cannot read {}", src.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        writer
            .write_all(&buf[..n])
            .with_context(|| format!("Cannot write {}", dst.display()))?;
    }
    writer.flush().with_context(|| format!("Cannot flush {}", dst.display()))?;
    drop(writer);
    Ok(hex::encode(hasher.finalize()))
}

/// Rejects a single ISO member that FAT32 cannot hold, before any copying.
/// Boot-partition extraction dies halfway through a multi-GB copy otherwise.
fn fits_fat32(display: &str, size: u64) -> Result<()> {
    const FAT32_MAX_FILE: u64 = 4_294_967_295;
    if size > FAT32_MAX_FILE {
        bail!(
            "This OS image contains '{}' ({} bytes), which exceeds FAT32's 4 GB per-file limit. \
             Ferry cannot make a bootable USB from it by extraction.",
            display,
            size
        );
    }
    Ok(())
}

fn mount_iso(iso_path: &Path) -> Result<char> {
    let script = "param([string]$IsoPath)\n\
        $ErrorActionPreference = 'Stop'\n\
        $img = Mount-DiskImage -ImagePath $IsoPath -PassThru\n\
        $vol = $img | Get-Volume\n\
        if (-not $vol.DriveLetter) { throw 'Mounted ISO has no drive letter' }\n\
        Write-Output $vol.DriveLetter\n";
    let iso_str = iso_path.to_string_lossy().to_string();
    let out = run_powershell_script(script, &["-IsoPath", &iso_str]).context("Failed to mount ISO")?;
    let trimmed = out.trim();
    let mut chars = trimmed.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => Ok(c.to_ascii_uppercase()),
        _ => bail!("Could not determine the mounted ISO's drive letter (got: '{trimmed}')"),
    }
}

fn dismount_iso(iso_path: &Path) -> Result<()> {
    let script = "param([string]$IsoPath)\n\
        $ErrorActionPreference = 'Stop'\n\
        Dismount-DiskImage -ImagePath $IsoPath | Out-Null\n";
    let iso_str = iso_path.to_string_lossy().to_string();
    run_powershell_script(script, &["-IsoPath", &iso_str]).context("Failed to dismount ISO")?;
    Ok(())
}

/// Runs a PowerShell script from a temp file with positional named args
/// (never string-interpolated into the command line) so arbitrary path
/// content can't be interpreted as script code.
fn run_powershell_script(script: &str, args: &[&str]) -> Result<String> {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "ferry-iso-{}.ps1",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    {
        let mut f = std::fs::File::create(&path).context("Could not create PowerShell script file")?;
        f.write_all(script.as_bytes())
            .context("Could not write PowerShell script")?;
        f.flush().context("Could not flush PowerShell script")?;
    }

    let mut cmd = crate::proc::hidden("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]);
    cmd.arg(&path);
    cmd.args(args);
    let result = cmd.output().context("Failed to run PowerShell");

    let _ = std::fs::remove_file(&path);

    let output = result?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("PowerShell script failed: {} {}", stdout.trim(), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::fits_fat32;

    #[test]
    fn ubuntu_sized_members_pass() {
        // Real measurement: Ubuntu 24.04.2's largest member is 1.69 GB.
        assert!(fits_fat32("casper/minimal.squashfs", 1_690_000_000).is_ok());
    }

    #[test]
    fn windows_sized_wim_fails_before_copying() {
        assert!(fits_fat32("sources/install.wim", 4_700_000_000).is_err());
    }

    #[test]
    fn exactly_4gib_minus_one_passes() {
        assert!(fits_fat32("edge.bin", 4_294_967_295).is_ok());
        assert!(fits_fat32("edge.bin", 4_294_967_296).is_err());
    }

    #[test]
    fn copy_hash_round_trip_matches() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.bin");
        let dst = dir.path().join("b.bin");
        std::fs::write(&src, b"ferry-bootloader-test-content-12345").unwrap();
        let h1 = super::hash_while_copying(&src, &dst).unwrap();
        let h2 = crate::hashing::hash_file(&dst).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn corrupted_destination_is_detected() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.bin");
        let dst = dir.path().join("b.bin");
        std::fs::write(&src, b"original-content-here").unwrap();
        let h1 = super::hash_while_copying(&src, &dst).unwrap();
        // Same length, different bytes: the old size check would pass this.
        std::fs::write(&dst, b"corrupted-content-her").unwrap();
        let h2 = crate::hashing::hash_file(&dst).unwrap();
        assert_ne!(h1, h2);
    }
}
