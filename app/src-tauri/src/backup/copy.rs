/// Copy files from the source list to the USB Backup/ directory.
/// Emits progress events so the frontend can show a live progress bar.
use crate::backup::scan::FileToBackup;
use crate::safety::{
    ensure_inside, ensure_source_under_home, sanitize_relative_path, validate_usb_root,
};
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks

/// Tauri command: copy `files` into `<usb_root>/Backup/`.
/// Emits `backup:progress` events during copy.
#[tauri::command]
pub async fn copy_files_to_usb(
    app: AppHandle,
    files: Vec<FileToBackup>,
    usb_root: String,
) -> Result<(), String> {
    copy_files(app, files, PathBuf::from(usb_root))
        .await
        .map_err(|e| e.to_string())
}

async fn copy_files(
    app: AppHandle,
    files: Vec<FileToBackup>,
    usb_root: PathBuf,
) -> Result<()> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let backup_root = canonical_usb.join("Backup");
    let total = files.len() as u64;

    for (i, file) in files.iter().enumerate() {
        let relative = sanitize_relative_path(&file.relative)?;
        let source = ensure_source_under_home(&file.source)?;
        let dest = backup_root.join(&relative);
        ensure_inside(&canonical_usb, &dest)?;

        // Create parent directories preserving the original folder structure.
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Cannot create directory: {:?}", parent))?;
        }

        copy_file_chunked(&source, &dest)
            .with_context(|| format!("Failed to copy {:?}", source))?;

        // Emit progress to frontend.
        let _ = app.emit("backup:progress", serde_json::json!({
            "stage": "copy",
            "current": i as u64 + 1,
            "total": total,
            "current_item": file.source.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        }));
    }

    Ok(())
}

/// Copy a single file in 1 MB chunks.
fn copy_file_chunked(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    let mut src_file = std::fs::File::open(src)
        .with_context(|| format!("Cannot open source file: {:?}", src))?;
    let mut dst_file = std::fs::File::create(dst)
        .with_context(|| format!("Cannot create dest file: {:?}", dst))?;

    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = src_file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst_file.write_all(&buf[..n])?;
    }
    Ok(())
}

