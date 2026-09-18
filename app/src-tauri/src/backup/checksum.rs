/// SHA-256 checksum every file in Backup/, write manifest.json, and verify.
/// The manifest is the gate that must pass before the erase step is unlocked.
use crate::backup::scan::FileToBackup;
use crate::safety::{
    ensure_inside, ensure_source_under_home, sanitize_relative_path, validate_usb_root,
};
use crate::types::{Manifest, ManifestEntry, SkippedEntry};
use crate::AppState;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

/// Tauri command: hash every file in the Backup/ folder and write manifest.json.
/// Returns the serialised manifest on success.
#[tauri::command]
pub async fn verify_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<FileToBackup>,
    usb_root: String,
    skipped: Vec<(String, String)>,
) -> Result<Manifest, String> {
    build_and_verify_manifest(app, state, files, PathBuf::from(usb_root), skipped)
        .await
        .map_err(|e| e.to_string())
}

async fn build_and_verify_manifest(
    app: AppHandle,
    state: State<'_, AppState>,
    files: Vec<FileToBackup>,
    usb_root: PathBuf,
    skipped_raw: Vec<(String, String)>,
) -> Result<Manifest> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let backup_root = canonical_usb.join("Backup");
    let total = files.len() as u64;
    let mut entries: Vec<ManifestEntry> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    for (i, file) in files.iter().enumerate() {
        let relative = match sanitize_relative_path(&file.relative) {
            Ok(relative) => relative,
            Err(e) => {
                failures.push(format!("{}: {}", file.source.display(), e));
                continue;
            }
        };
        let source = match ensure_source_under_home(&file.source) {
            Ok(source) => source,
            Err(e) => {
                failures.push(format!("{}: {}", file.source.display(), e));
                continue;
            }
        };
        let backup_path = backup_root.join(&relative);
        if let Err(e) = ensure_inside(&canonical_usb, &backup_path) {
            failures.push(format!("{}: {}", file.source.display(), e));
            continue;
        }

        match hash_file(&backup_path) {
            Ok(hash) => {
                // Also hash the source to confirm it hasn't changed since copy.
                let source_hash = hash_file(&source).unwrap_or_default();
                if hash != source_hash {
                    failures.push(format!(
                        "Checksum mismatch: {} (source and backup differ)",
                        file.source.display()
                    ));
                } else {
                    entries.push(ManifestEntry {
                        original_path: source.to_string_lossy().to_string(),
                        backup_path: relative.to_string_lossy().replace('\\', "/"),
                        size_bytes: file.size_bytes,
                        sha256: hash,
                    });
                }
            }
            Err(e) => {
                failures.push(format!("{}: {}", file.source.display(), e));
            }
        }

        let _ = app.emit("backup:progress", serde_json::json!({
            "stage": "verify",
            "current": i as u64 + 1,
            "total": total,
            "current_item": file.source.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        }));
    }

    if !failures.is_empty() {
        bail!("Verification failed for {} file(s):\n{}", failures.len(), failures.join("\n"));
    }

    let source_user = std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string());
    let skipped = skipped_raw
        .into_iter()
        .map(|(path, reason)| SkippedEntry { path, reason })
        .collect();

    let manifest = Manifest {
        ferry_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now().to_rfc3339(),
        source_user,
        source_os: get_os_version(),
        files: entries,
        skipped,
    };

    // Write manifest.json into Backup/ (will be encrypted with the rest).
    let manifest_path = backup_root.join("manifest.json");
    let json = serde_json::to_string_pretty(&manifest)
        .context("Failed to serialise manifest")?;
    std::fs::write(&manifest_path, json)
        .context("Failed to write manifest.json")?;

    // Unlock the erase step for this exact USB root in this backend session.
    // The erase command consumes this one-time token.
    if let Ok(mut verified) = state.verified_roots.lock() {
        verified.insert(canonical_usb.to_string_lossy().to_string());
    }

    Ok(manifest)
}

pub fn hash_file(path: &PathBuf) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Cannot open for hashing: {:?}", path))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn get_os_version() -> String {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "(Get-WmiObject Win32_OperatingSystem).Caption"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn hash_is_deterministic() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"ferry test content").unwrap();
        let path = f.path().to_path_buf();
        let h1 = hash_file(&path).unwrap();
        let h2 = hash_file(&path).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex is 64 chars
    }
}

