/// User-supplied OS images: drag-and-drop or browse for an `.iso` file instead
/// of downloading one. The image is taken as-is — there is no vendor checksum
/// to check it against, so the UI must label it user-supplied/unverified.
/// Everything else (removable-USB containment, safe filenames) is enforced
/// exactly like the download path.
use crate::safety::{sanitize_filename, validate_usb_root};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks

#[derive(Debug, Clone, Serialize)]
pub struct CustomIsoInfo {
    pub filename: String,
    pub size_bytes: u64,
}

fn validated_source(path: &str) -> Result<(PathBuf, String, u64)> {
    let source = PathBuf::from(path);
    let meta = std::fs::metadata(&source)
        .with_context(|| "OS image file not found — did it move?".to_string())?;
    if !meta.is_file() {
        bail!("That is not a file — drop a single .iso image.");
    }
    let raw_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !raw_name.to_lowercase().ends_with(".iso") {
        bail!("Only .iso images are supported.");
    }
    let filename = sanitize_filename(raw_name)?;
    let size = meta.len();
    if size == 0 {
        bail!("That .iso file is empty.");
    }
    Ok((source, filename, size))
}

/// Tauri command: inspect a dropped/browsed file. Returns name + size for
/// display and for sizing the boot partition. No copying yet.
#[tauri::command]
pub async fn inspect_custom_iso(path: String) -> Result<CustomIsoInfo, String> {
    let (_, filename, size_bytes) =
        validated_source(&path).map_err(|e| e.to_string())?;
    Ok(CustomIsoInfo {
        filename,
        size_bytes,
    })
}

/// Tauri command: copy a validated `.iso` onto the USB data partition,
/// emitting `download:progress` events so the existing progress UI works
/// unchanged. Returns the staged filename for the bootloader step.
#[tauri::command]
pub async fn copy_custom_iso(
    app: AppHandle,
    source_path: String,
    dest_dir: String,
) -> Result<String, String> {
    copy_iso(app, &source_path, &dest_dir)
        .await
        .map_err(|e| e.to_string())
}

async fn copy_iso(app: AppHandle, source_path: &str, dest_dir: &str) -> Result<String> {
    let (source, filename, size) = validated_source(source_path)?;
    let (canonical_dest, _) = validate_usb_root(dest_dir)?;
    let dest_path = canonical_dest.join(&filename);

    let mut src = std::fs::File::open(&source).context("Cannot open the .iso file")?;
    let out = std::fs::File::create(&dest_path)
        .with_context(|| format!("Cannot write to {}", dest_path.display()))?;
    let mut writer = std::io::BufWriter::new(out);
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut written: u64 = 0;
    loop {
        let n = std::io::Read::read(&mut src, &mut buf).context("Read failed")?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&buf[..n])
            .context("Write to USB failed — is the drive still connected?")?;
        written += n as u64;
        let _ = app.emit("download:progress", serde_json::json!({
            "stage": "download",
            "current": written,
            "total": size,
            "current_item": filename,
        }));
    }
    writer.flush()?;
    Ok(filename)
}

/// Resolve `dest_dir` for tests without touching a real USB.
#[cfg(test)]
fn resolve_dest_for_test(dest_dir: &std::path::Path) -> Result<PathBuf> {
    validate_usb_root(&dest_dir.to_string_lossy()).map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_iso(dir: &std::path::Path, name: &str, bytes: &[u8]) -> String {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn inspect_accepts_iso_and_reports_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = sample_iso(dir.path(), "ubuntu-24.04.2-desktop-amd64.iso", b"fake-iso-bytes");
        let info = validated_source(&path).map(|(_, name, size)| (name, size)).unwrap();
        assert_eq!(info.0, "ubuntu-24.04.2-desktop-amd64.iso");
        assert_eq!(info.1, 14);
    }

    #[test]
    fn inspect_rejects_non_iso_missing_and_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(validated_source(&sample_iso(dir.path(), "notes.txt", b"x")).is_err());
        assert!(validated_source(&dir.path().join("ghost.iso").to_string_lossy()).is_err());
        assert!(validated_source(&sample_iso(dir.path(), "empty.iso", b"")).is_err());
    }

    #[test]
    fn usb_roots_are_still_required_for_staging() {
        // A plain temp dir is not removable media → staging must refuse it.
        let dir = tempfile::tempdir().unwrap();
        assert!(resolve_dest_for_test(dir.path()).is_err());
    }
}
