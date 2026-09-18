/// Download an OS ISO to the USB data partition with resume support.
/// Emits download:progress events. Verifies the checksum after download;
/// deletes the file and returns an error if verification fails.
use crate::download::sources::resolve_source;
use crate::safety::{sanitize_filename, validate_usb_root};
use anyhow::{bail, Context, Result};
use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

/// Tauri command: download the OS image for `source_id` to `<dest_dir>/<filename>`.
/// The URL and checksum always come from the bundled manifest; the frontend only
/// selects `source_id` and the already-validated USB destination.
/// Resumes partial downloads using HTTP Range requests.
#[tauri::command]
pub async fn download_os_image(
    app: AppHandle,
    source_id: String,
    dest_dir: String,
) -> Result<String, String> {
    let dest = PathBuf::from(dest_dir);
    download(app, &source_id, dest)
        .await
        .map_err(|e| e.to_string())
}

async fn download(
    app: AppHandle,
    source_id: &str,
    dest_dir: PathBuf,
) -> Result<String> {
    let source = resolve_source(source_id)?;
    let url = source
        .url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("OS source '{}' has no direct download URL", source_id))?;
    if !crate::safety::is_http_url(url) {
        bail!("OS source '{}' has an invalid download URL", source_id);
    }
    let dest_str = dest_dir.to_string_lossy().to_string();
    let (canonical_dest, _) = validate_usb_root(&dest_str)?;
    let client = Client::new();

    // Derive a safe filename from the bundled URL (strip query/fragment first).
    let last_segment = url.rsplit('/').next().unwrap_or("os-image.iso");
    let clean_segment = last_segment
        .split(['?', '#'])
        .next()
        .unwrap_or("os-image.iso");
    let filename = sanitize_filename(clean_segment)?;
    let dest_path = canonical_dest.join(filename.clone());

    // Check for a partial download and set Range header if so.
    let mut existing_size = if dest_path.exists() {
        std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let mut response = if existing_size > 0 {
        client
            .get(url)
            .header("Range", format!("bytes={}-", existing_size))
            .send()
            .await
            .context("HTTP request failed")?
    } else {
        client.get(url).send().await.context("HTTP request failed")?
    };

    // If the server ignored Range (200 instead of 206), restart instead of
    // appending a second copy of the file to the partial download.
    if existing_size > 0 && response.status() != StatusCode::PARTIAL_CONTENT {
        existing_size = 0;
        response = client.get(url).send().await.context("HTTP request failed")?;
    }
    response.error_for_status_ref().context("HTTP request failed")?;
    let total = response
        .content_length()
        .map(|l| l + existing_size)
        .unwrap_or(0);

    // Open file for append (resume) or create/truncate.
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(existing_size > 0)
        .write(true)
        .truncate(existing_size == 0)
        .open(&dest_path)
        .context("Cannot open destination file")?;
    let mut writer = std::io::BufWriter::new(file);
    let mut downloaded = existing_size;

    while let Some(chunk) = response.chunk().await.context("Stream read failed")? {
        writer.write_all(&chunk).context("Write failed")?;
        downloaded += chunk.len() as u64;

        let _ = app.emit("download:progress", serde_json::json!({
            "stage": "download",
            "current": downloaded,
            "total": total,
            "current_item": filename,
        }));
    }
    writer.flush()?;

    // Resolve expected hash from the bundled manifest only.
    let expected = match source.checksum_url.as_deref() {
        Some(cu) => fetch_sha256_from_checksum_file(&client, cu, &filename).await?,
        None => bail!("No checksum available for {}; cannot verify download", source_id),
    };
    let expected = normalize_sha256(&expected)?;

    // Verify.
    let actual = hash_file_sha256(&dest_path)?;
    if actual != expected {
        std::fs::remove_file(&dest_path).ok();
        bail!(
            "Checksum mismatch for {}!\nExpected: {}\nGot:      {}\nThe file has been deleted. Please try again.",
            filename, expected, actual
        );
    }

    Ok(dest_path.to_string_lossy().to_string())
}

/// Fetch a checksum file and extract the hash for `filename`.
/// Handles `<hash><space><space?>[*]<filename>` formats (Ubuntu SHA256SUMS etc.).
async fn fetch_sha256_from_checksum_file(
    client: &Client,
    checksum_url: &str,
    filename: &str,
) -> Result<String> {
    if !crate::safety::is_http_url(checksum_url) {
        bail!("Invalid checksum URL for '{}'", filename);
    }
    let text = client
        .get(checksum_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next().unwrap_or("");
        let name = parts.collect::<Vec<_>>().join(" ");
        let name = name.trim().trim_start_matches('*');
        if !hash.is_empty() && name == filename {
            return normalize_sha256(hash);
        }
    }
    bail!("Could not find checksum for '{}' in {}", filename, checksum_url);
}

fn normalize_sha256(value: &str) -> Result<String> {
    let normalized = value.trim().to_lowercase();
    if normalized.len() != 64 || !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Invalid SHA-256 checksum format");
    }
    Ok(normalized)
}

fn hash_file_sha256(path: &PathBuf) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

