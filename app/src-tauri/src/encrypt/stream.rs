/// Streaming container format for Ferry encrypted backups.
///
/// Layout of `Backup.enc`:
///   [9-byte magic `FERRYENC1`][u32 BE chunk size][records...]
/// Each record is `[12-byte nonce][u32 BE ciphertext len][ciphertext+GCM tag]`
/// with one fresh random nonce per chunk. Plaintext is zipped to a temp file
/// first, then encrypted chunk-by-chunk, so multi-GB backups never sit fully
/// in RAM. Files without the magic header are rejected as legacy/foreign.
use crate::encrypt::keygen::KEY_LEN;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{bail, Context, Result};
use rand::RngCore;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub(crate) const ENC_MAGIC: &[u8; 9] = b"FERRYENC1";
/// Plaintext bytes per AEAD chunk.
pub(crate) const ENC_CHUNK_SIZE: usize = 1024 * 1024;
/// Sanity cap for a single record length (chunk + 16-byte GCM tag, generously).
const MAX_RECORD_LEN: usize = 64 * 1024 * 1024 + 16;

/// Zip `dir` into `dest` streaming file contents (no whole-archive buffering).
pub(crate) fn zip_directory_to_file(dir: &Path, dest: &Path) -> Result<()> {
    let file =
        std::fs::File::create(dest).with_context(|| format!("Cannot create {}", dest.display()))?;
    let mut zip = ZipWriter::new(std::io::BufWriter::new(file));
    let options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let name = path
            .strip_prefix(dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if path.is_dir() {
            if !name.is_empty() {
                zip.add_directory(&name, options)?;
            }
        } else {
            zip.start_file(&name, options)?;
            let mut f = std::fs::File::open(path)
                .with_context(|| format!("Cannot open {}", path.display()))?;
            std::io::copy(&mut f, &mut zip)
                .with_context(|| format!("Cannot compress {}", path.display()))?;
        }
    }

    let mut writer = zip.finish().context("Cannot finalize ZIP archive")?;
    writer.flush().context("Cannot flush ZIP archive")?;
    writer
        .get_mut()
        .sync_all()
        .context("Cannot flush ZIP archive to disk")?;
    Ok(())
}

/// Stream-encrypt `plaintext` into `dest_tmp` (caller renames into place).
pub(crate) fn encrypt_file_chunked(
    key_bytes: &[u8; KEY_LEN],
    plaintext: &Path,
    dest_tmp: &Path,
) -> Result<()> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut input = std::fs::File::open(plaintext)
        .with_context(|| format!("Cannot open {}", plaintext.display()))?;
    let out_file = std::fs::File::create(dest_tmp)
        .with_context(|| format!("Cannot create {}", dest_tmp.display()))?;
    let mut out = std::io::BufWriter::new(out_file);
    out.write_all(ENC_MAGIC)?;
    out.write_all(&(ENC_CHUNK_SIZE as u32).to_be_bytes())?;

    let mut buf = vec![0u8; ENC_CHUNK_SIZE];
    loop {
        let n = read_up_to(&mut input, &mut buf)?;
        if n == 0 {
            break;
        }
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, &buf[..n])
            .map_err(|e| anyhow::anyhow!("AES-GCM encryption failed: {}", e))?;
        out.write_all(&nonce_bytes)?;
        out.write_all(&(ciphertext.len() as u32).to_be_bytes())?;
        out.write_all(&ciphertext)?;
    }
    out.flush().context("Cannot flush encrypted output")?;
    out.get_mut()
        .sync_all()
        .context("Cannot flush encrypted output to disk")?;
    Ok(())
}

/// Stream-decrypt `ciphertext` into `dest`, verifying the container header and
/// every chunk's GCM tag along the way.
pub(crate) fn decrypt_file_chunked(
    key_bytes: &[u8; KEY_LEN],
    ciphertext: &Path,
    dest: &Path,
) -> Result<()> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let mut input = std::fs::File::open(ciphertext)
        .with_context(|| format!("Cannot open {}", ciphertext.display()))?;

    let mut magic = [0u8; 9];
    input.read_exact(&mut magic).context(
        "Backup.enc is too small to be valid (missing container header)",
    )?;
    if &magic != ENC_MAGIC {
        bail!("Unsupported backup format: missing Ferry container header");
    }
    let mut chunk_size_raw = [0u8; 4];
    input
        .read_exact(&mut chunk_size_raw)
        .context("Backup.enc header is truncated")?;
    let chunk_size = u32::from_be_bytes(chunk_size_raw) as usize;
    if chunk_size == 0 || chunk_size > MAX_RECORD_LEN {
        bail!("Backup.enc header has an invalid chunk size");
    }

    let out_file =
        std::fs::File::create(dest).with_context(|| format!("Cannot create {}", dest.display()))?;
    let mut out = std::io::BufWriter::new(out_file);
    loop {
        // Read one nonce; clean EOF here means a complete container.
        let mut nonce_bytes = [0u8; 12];
        let mut filled = 0;
        while filled < 12 {
            match input.read(&mut nonce_bytes[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(e) => return Err(e).context("Cannot read Backup.enc"),
            }
        }
        if filled == 0 {
            break;
        }
        if filled != 12 {
            bail!("Backup.enc is truncated");
        }

        let mut len_raw = [0u8; 4];
        input
            .read_exact(&mut len_raw)
            .context("Backup.enc record is truncated")?;
        let record_len = u32::from_be_bytes(len_raw) as usize;
        if record_len == 0 || record_len > MAX_RECORD_LEN {
            bail!("Backup.enc record has an invalid length");
        }
        let mut record = vec![0u8; record_len];
        input
            .read_exact(&mut record)
            .context("Backup.enc record is truncated")?;

        let nonce = Nonce::from_slice(&nonce_bytes);
        let plaintext = cipher.decrypt(nonce, record.as_slice()).map_err(|_| {
            anyhow::anyhow!("Decryption failed. The password is incorrect or the backup is corrupted.")
        })?;
        out.write_all(&plaintext)
            .context("Cannot write decrypted output")?;
    }
    out.flush().context("Cannot flush decrypted output")?;
    out.get_mut()
        .sync_all()
        .context("Cannot flush decrypted output to disk")?;
    Ok(())
}

/// Confirm a decrypted ZIP file contains a parseable Ferry manifest.
pub(crate) fn check_manifest_in_zip(zip_path: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)
        .with_context(|| format!("Cannot open {}", zip_path.display()))?;
    let mut archive =
        zip::ZipArchive::new(file).context("Backup data is not a valid ZIP archive")?;
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .context("Backup data is missing manifest.json")?
        .read_to_string(&mut manifest)
        .context("Cannot read manifest.json from backup data")?;
    let _: crate::types::Manifest =
        serde_json::from_str(&manifest).context("Backup manifest is invalid")?;
    Ok(())
}

fn read_up_to(reader: &mut std::fs::File, buf: &mut [u8]) -> Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match reader.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) => return Err(e).context("Cannot read input file"),
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::keygen::derive_key;

    #[test]
    fn chunked_round_trip_preserves_bytes() {
        let dir = tempfile::tempdir().unwrap();
        // 2.5 MB of deterministic data forces multiple 1 MB chunks.
        let big: Vec<u8> = (0..2_500_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(dir.path().join("big.bin"), &big).unwrap();
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"ferry_version":"test","created_at":"t","source_user":"u","source_os":"o","files":[],"skipped":[]}"#,
        )
        .unwrap();

        let zip_path = dir.path().join("test.zip");
        zip_directory_to_file(dir.path(), &zip_path).unwrap();

        let (key, _) = derive_key("test-password-123").unwrap();
        let enc_path = dir.path().join("test.enc");
        let enc_tmp = dir.path().join("test.enc.tmp");
        encrypt_file_chunked(&key, &zip_path, &enc_tmp).unwrap();
        std::fs::rename(&enc_tmp, &enc_path).unwrap();

        let dec_path = dir.path().join("test.dec.zip");
        decrypt_file_chunked(&key, &enc_path, &dec_path).unwrap();
        check_manifest_in_zip(&dec_path).unwrap();

        let original = std::fs::read(&zip_path).unwrap();
        let round_tripped = std::fs::read(&dec_path).unwrap();
        assert_eq!(original, round_tripped);
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        let zip_path = dir.path().join("t.zip");
        zip_directory_to_file(dir.path(), &zip_path).unwrap();
        let (key, _) = derive_key("correct-horse-pw-1").unwrap();
        let (wrong, _) = derive_key("wrong-horse-pw-22").unwrap();
        let enc_path = dir.path().join("t.enc");
        encrypt_file_chunked(&key, &zip_path, &enc_path).unwrap();
        let dec_path = dir.path().join("t.dec");
        assert!(decrypt_file_chunked(&wrong, &enc_path, &dec_path).is_err());
    }

    #[test]
    fn legacy_header_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy.enc");
        std::fs::write(&legacy, b"\x00\x01\x02not a ferry container").unwrap();
        let out = dir.path().join("out.zip");
        let (key, _) = derive_key("test-password-123").unwrap();
        assert!(decrypt_file_chunked(&key, &legacy, &out).is_err());
    }
}
