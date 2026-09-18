/// Decrypt Backup.enc back into a Backup/ folder (for restore).
/// Reads the salt from backup.salt, re-derives the key, stream-decrypts the
/// chunked AES-GCM container to a temp ZIP, then safely unzips the result
/// into a staging directory.
use crate::crypto::keygen::derive_key_with_salt;
use crate::crypto::stream::decrypt_file_chunked;
use crate::safety::{validate_staging_dir, validate_usb_root};
use anyhow::{Context, Result};
use std::path::PathBuf;
use zeroize::Zeroize;

/// Tauri command: decrypt Backup.enc to a staging Backup/ directory.
/// Returns the path to the decrypted Backup/ directory.
#[tauri::command]
pub async fn decrypt_backup(
    usb_root: String,
    password: String,
    staging_dir: String,
) -> Result<String, String> {
    let usb = PathBuf::from(&usb_root);
    let staging = PathBuf::from(&staging_dir);
    run_decrypt(usb, &password, staging)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

fn run_decrypt(usb_root: PathBuf, password: &str, staging: PathBuf) -> Result<PathBuf> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    // An empty staging dir means "use a fresh OS temp folder", so the UI never
    // has to discover the temp path itself.
    let staging = if staging.as_os_str().is_empty() {
        let dir = std::env::temp_dir().join(format!(
            "ferry-restore-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).context("Cannot create restore staging directory")?;
        dir
    } else {
        staging
    };
    let staging_str = staging.to_string_lossy().to_string();
    let canonical_staging = validate_staging_dir(&staging_str)?;
    let enc_path = canonical_usb.join("Backup.enc");
    let salt_path = canonical_usb.join("backup.salt");
    let zip_tmp_path = canonical_staging.join("Backup.zip.tmp");
    let _ = std::fs::remove_file(&zip_tmp_path);

    // 1. Re-derive the key from password + stored salt.
    let salt = std::fs::read_to_string(&salt_path)
        .context("Could not read backup.salt — is this a valid Ferry backup?")?;
    let mut key_bytes = derive_key_with_salt(password, salt.trim())
        .context("Key derivation failed")?;

    // 2. Stream-decrypt the chunked container to a temp ZIP file.
    let decrypted = decrypt_file_chunked(&key_bytes, &enc_path, &zip_tmp_path).map_err(|e| {
        anyhow::anyhow!(
            "Decryption failed. The password is incorrect or the backup is corrupted. ({})",
            e
        )
    });
    key_bytes.zeroize();
    decrypted?;
    crate::crypto::stream::check_manifest_in_zip(&zip_tmp_path)
        .context("Decrypted backup is missing a valid manifest")?;

    // 3. Unzip to staging directory, rejecting absolute/parent paths.
    let file = std::fs::File::open(&zip_tmp_path).context("Cannot open decrypted backup")?;
    let mut archive = zip::ZipArchive::new(file)
        .context("Decrypted data is not a valid ZIP archive")?;

    let out_dir = canonical_staging.join("Backup");
    std::fs::create_dir_all(&out_dir).context("Cannot create restore staging directory")?;
    let out_canon = out_dir.canonicalize().context("Cannot resolve staging directory")?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Invalid ZIP entry")?;
        let safe_path = file.enclosed_name().context("ZIP entry has an unsafe path")?;
        let dest = out_canon.join(safe_path);
        crate::safety::ensure_inside(&out_canon, &dest)?;
        if file.is_dir() {
            std::fs::create_dir_all(&dest).context("Cannot create staging directory")?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).context("Cannot create staging directory")?;
            }
            let mut out_file =
                std::fs::File::create(&dest).context("Cannot write staging file")?;
            std::io::copy(&mut file, &mut out_file).context("Cannot extract staging file")?;
        }
    }
    let _ = std::fs::remove_file(&zip_tmp_path);

    Ok(out_canon)
}
