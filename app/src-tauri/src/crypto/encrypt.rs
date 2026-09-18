/// Encrypt the entire Backup/ folder into a single Backup.enc file.
/// Streaming design: Backup/ is zipped to a temp file, then AES-256-GCM
/// encrypted in 1 MB chunks, so multi-GB backups never sit fully in RAM.
/// The plaintext Backup/ folder is deleted only after verification.
use crate::crypto::keygen::derive_key;
use crate::crypto::stream::{
    check_manifest_in_zip, decrypt_file_chunked, encrypt_file_chunked, zip_directory_to_file,
};
use crate::safety::validate_usb_root;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use zeroize::Zeroize;

/// Tauri command: zip then AES-256-GCM encrypt Backup/ → Backup.enc.
/// Stores the salt in backup.salt. Deletes plaintext Backup/ on success.
#[tauri::command]
pub async fn encrypt_backup(
    usb_root: String,
    password: String,
) -> Result<(), String> {
    run_encrypt(PathBuf::from(usb_root), &password)
        .map_err(|e| e.to_string())
}

fn run_encrypt(usb_root: PathBuf, password: &str) -> Result<()> {
    if password.trim().len() < 12 {
        bail!("Encryption password must be at least 12 characters long");
    }
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let backup_dir = canonical_usb.join("Backup");
    let enc_path = canonical_usb.join("Backup.enc");
    let enc_tmp_path = canonical_usb.join("Backup.enc.tmp");
    let zip_tmp_path = canonical_usb.join("Backup.zip.tmp");
    let verify_tmp_path = canonical_usb.join("Backup.verify.tmp");
    let salt_path = canonical_usb.join("backup.salt");
    if !backup_dir.is_dir() {
        bail!("Backup/ folder not found; run backup and verification first");
    }
    for stale in [&enc_tmp_path, &zip_tmp_path, &verify_tmp_path] {
        let _ = std::fs::remove_file(stale);
    }

    // 1. Derive key and save salt.
    let (mut key_bytes, salt) = derive_key(password)?;
    std::fs::write(&salt_path, salt.as_bytes())
        .context("Could not write backup.salt")?;

    // 2. ZIP the Backup/ folder to a temp file (streamed, not buffered).
    let zip_result = zip_directory_to_file(&backup_dir, &zip_tmp_path)
        .context("Failed to compress Backup/ folder");
    // 3. Encrypt the temp ZIP chunk-by-chunk into Backup.enc.tmp.
    let enc_result = zip_result.and_then(|_| {
        encrypt_file_chunked(&key_bytes, &zip_tmp_path, &enc_tmp_path)
            .context("Failed to encrypt Backup/ folder")
    });
    // The plaintext ZIP temp file is no longer needed either way.
    let _ = std::fs::remove_file(&zip_tmp_path);
    enc_result?;
    std::fs::rename(&enc_tmp_path, &enc_path).context("Could not finalize Backup.enc")?;

    // 4. Verify by stream-decrypting the finished file and checking the manifest.
    let verified = decrypt_file_chunked(&key_bytes, &enc_path, &verify_tmp_path)
        .context("Backup.enc verification failed")
        .and_then(|_| {
            check_manifest_in_zip(&verify_tmp_path).context("Backup.enc verification failed")
        });
    let _ = std::fs::remove_file(&verify_tmp_path);
    key_bytes.zeroize();
    verified.context("Backup.enc verification failed; plaintext Backup/ was kept")?;

    // 5. Remove the plaintext Backup/ folder only after verification.
    std::fs::remove_dir_all(&backup_dir)
        .context("Could not delete plaintext Backup/ folder after encryption")?;

    Ok(())
}
