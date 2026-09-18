/// Bootloader: copies a pre-built UEFI bootloader binary onto the FAT32 boot partition.
/// At MVP the bootloader is a minimal GRUB2 EFI binary bundled with Ferry.
use crate::safety::{
    ensure_inside, ensure_removable_drive_letter, validate_drive_letter,
};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// Tauri command: write the bundled UEFI bootloader to the FAT32 boot partition.
/// `boot_partition_letter` is the drive letter of the newly created FAT32 partition.
#[tauri::command]
pub async fn write_bootloader(boot_partition_letter: String) -> Result<(), String> {
    install_bootloader(&boot_partition_letter)
        .map_err(|e| e.to_string())
}

fn install_bootloader(partition: &str) -> Result<()> {
    let letter = validate_drive_letter(partition)?;
    ensure_removable_drive_letter(letter)?;
    let root = PathBuf::from(format!("{}:\\", letter));
    let root_canon = root
        .canonicalize()
        .context("Boot partition is not available")?;
    let dest_efi = root_canon.join("EFI").join("BOOT");
    std::fs::create_dir_all(&dest_efi)
        .context("Could not create EFI/BOOT directory on boot partition")?;
    let dest_canon = dest_efi
        .canonicalize()
        .context("Could not resolve EFI/BOOT directory")?;
    ensure_inside(&root_canon, &dest_canon)?;

    // The bundled GRUB EFI binary lives at assets/bootx64.efi relative to the
    // src-tauri directory, and is read at runtime so Ferry never ships a fake
    // bootloader as if it were bootable. If it is absent, we say so clearly.
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/bootx64.efi");
    if !src.exists() {
        bail!(
            "Bootloader file not found at {}. Add a real GRUB/UEFI bootx64.efi there before writing a bootable USB.",
            src.display()
        );
    }

    let bootloader_bytes = std::fs::read(&src)
        .with_context(|| format!("Could not read bootloader at {}", src.display()))?;
    let dest_file = dest_canon.join("BOOTX64.EFI");
    std::fs::write(&dest_file, bootloader_bytes)
        .context("Could not write bootloader to boot partition")?;

    Ok(())
}

