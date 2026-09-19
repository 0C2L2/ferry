/// Ferry Cloud Backup — free, managed, on Ferry's own Backblaze B2 account.
///
/// The desktop app never holds Ferry's B2 master key. Instead, for every
/// upload or restore it asks the `assist-server` sidecar (see
/// ../../../../assist-server/src/services/b2admin.ts) for a brand-new,
/// disposable B2 Application Key scoped to exactly one backup's folder in
/// the bucket, one capability (write for upload, read for restore), and a
/// short lifetime. Even a leaked key can't touch any other backup.
///
/// Each backup lives at `<backup_id>/Backup.enc` and `<backup_id>/backup.salt`
/// in Ferry's shared bucket — the ID (a UUID minted server-side) is the only
/// thing the user needs to write down alongside their password to recover a
/// cloud copy if the physical USB is lost. After a successful restore, the
/// desktop app asks the backend to delete both files.
///
/// Architecture:
///   • The *already-encrypted* `Backup.enc` (plus the small `backup.salt`) is
///     uploaded as-is. B2 never sees plaintext — the AES-256-GCM layer is
///     Ferry's own.
///   • Upload is chunked (100 MB parts) so multi-GB files resume if the
///     connection drops mid-way.
///   • Progress is reported via Tauri events so the frontend shows a live
///     progress bar identically to the local backup stage.
///
/// B2 Large File API flow (per file > 5 MB):
///   1. b2_authorize_account  → auth_token + upload URL stem
///   2. b2_start_large_file   → file_id
///   3. b2_get_upload_part_url (per part) → part upload URL + token
///   4. POST each 100 MB chunk, collect SHA1 per part
///   5. b2_finish_large_file  → final file ID
///
/// For files ≤ 5 MB, b2_upload_file is used instead (no multipart).
use crate::cloud::progress::CloudProgress;
use crate::safety::validate_backup_dir;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

const PART_SIZE: u64 = 100 * 1024 * 1024; // 100 MB per B2 large-file part
const SMALL_FILE_LIMIT: u64 = 5 * 1024 * 1024; // ≤ 5 MB → single-shot upload

fn assist_server_url() -> String {
    std::env::var("FERRY_ASSIST_SERVER_URL").unwrap_or_else(|_| "http://localhost:8787".to_string())
}

// ── Tauri commands ───────────────────────────────────────────────────────────

/// Upload `<usb_root>/Backup.enc` + `backup.salt` to Ferry's managed B2
/// storage. Returns the backup ID the user should write down.
#[tauri::command]
pub async fn upload_backup_b2(app: AppHandle, usb_root: String) -> Result<String, String> {
    run_upload(app, PathBuf::from(usb_root)).await.map_err(|e| e.to_string())
}

/// Download a previously-uploaded backup into a fresh staging directory.
/// Returns that directory's path (pass it as `usb_root` to `decrypt_backup`).
#[tauri::command]
pub async fn download_backup_b2(backup_id: String) -> Result<String, String> {
    run_download(backup_id).await.map_err(|e| e.to_string())
}

/// Deletes a backup from Ferry's cloud storage — call once restore succeeds.
#[tauri::command]
pub async fn delete_cloud_backup(backup_id: String) -> Result<(), String> {
    run_delete(backup_id).await.map_err(|e| e.to_string())
}

// ── Backend key exchange ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopedKey {
    backup_id: String,
    key_id: String,
    application_key: String,
    bucket_id: String,
    bucket_name: String,
}

async fn fetch_upload_key(client: &Client) -> Result<ScopedKey> {
    client
        .post(format!("{}/api/cloud/upload-key", assist_server_url()))
        .send()
        .await
        .context("Could not reach the Ferry Cloud service")?
        .error_for_status()
        .context("Ferry Cloud service rejected the upload request")?
        .json()
        .await
        .context("Ferry Cloud service returned an unexpected response")
}

async fn fetch_download_key(client: &Client, backup_id: &str) -> Result<ScopedKey> {
    client
        .post(format!("{}/api/cloud/download-key", assist_server_url()))
        .json(&serde_json::json!({ "backupId": backup_id }))
        .send()
        .await
        .context("Could not reach the Ferry Cloud service")?
        .error_for_status()
        .context("Ferry Cloud service rejected the restore request — check the backup ID")?
        .json()
        .await
        .context("Ferry Cloud service returned an unexpected response")
}

async fn run_delete(backup_id: String) -> Result<()> {
    let client = Client::new();
    client
        .post(format!("{}/api/cloud/delete", assist_server_url()))
        .json(&serde_json::json!({ "backupId": backup_id }))
        .send()
        .await
        .context("Could not reach the Ferry Cloud service")?
        .error_for_status()
        .context("Ferry Cloud service could not delete the backup")?;
    Ok(())
}

// ── Upload ───────────────────────────────────────────────────────────────────

async fn run_upload(app: AppHandle, usb_root: PathBuf) -> Result<String> {
    let usb_root_str = usb_root.to_string_lossy().to_string();
    let canonical_usb = validate_backup_dir(&usb_root_str)?;
    let enc_path = canonical_usb.join("Backup.enc");
    let salt_path = canonical_usb.join("backup.salt");
    if !enc_path.exists() || !salt_path.exists() {
        bail!("Backup.enc / backup.salt not found — run local backup and encryption first");
    }

    let client = Client::new();
    let key = fetch_upload_key(&client).await?;
    let auth = authorize(&client, &key.key_id, &key.application_key).await?;

    // The small salt file first (near-instant), then the real backup.
    upload_one_file(
        &client,
        &auth,
        &key.bucket_id,
        &format!("{}/backup.salt", key.backup_id),
        &salt_path,
        &app,
        true, // silent: don't emit progress events for this tiny file
    )
    .await
    .context("Failed to upload backup.salt")?;

    let file_size = enc_path.metadata().context("Cannot read Backup.enc size")?.len();
    upload_one_file(
        &client,
        &auth,
        &key.bucket_id,
        &format!("{}/Backup.enc", key.backup_id),
        &enc_path,
        &app,
        false,
    )
    .await
    .context("Failed to upload Backup.enc")?;
    let _ = file_size;

    let _ = app.emit("cloud:done", serde_json::json!({ "backup_id": key.backup_id }));
    Ok(key.backup_id)
}

async fn upload_one_file(
    client: &Client,
    auth: &Auth,
    bucket_id: &str,
    file_name: &str,
    path: &Path,
    app: &AppHandle,
    silent: bool,
) -> Result<String> {
    let size = path.metadata().context("Cannot read file size")?.len();
    let ctx = UploadCtx {
        client,
        auth,
        bucket_id,
        file_name,
        path,
        app,
        api_url: &auth.api_url,
        silent,
    };
    if size <= SMALL_FILE_LIMIT {
        upload_small(&ctx, size).await
    } else {
        upload_large(&ctx, size).await
    }
}

// ── B2 API types ─────────────────────────────────────────────────────────────

// b2_authorize_account (v3) nests apiUrl/downloadUrl under apiInfo.storageApi,
// not at the top level — verified against a real live call, not assumed.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    authorization_token: String,
    api_info: ApiInfo,
}

#[derive(Debug, Deserialize)]
struct ApiInfo {
    #[serde(rename = "storageApi")]
    storage_api: StorageApi,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StorageApi {
    api_url: String,
    download_url: String,
}

struct Auth {
    token: String,
    api_url: String,
    download_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartLargeFileResponse {
    file_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetUploadPartUrlResponse {
    upload_url: String,
    authorization_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    file_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinishLargeFileRequest<'a> {
    file_id: &'a str,
    part_sha1_array: Vec<String>,
}

struct UploadCtx<'a> {
    client: &'a Client,
    auth: &'a Auth,
    bucket_id: &'a str,
    file_name: &'a str,
    path: &'a Path,
    app: &'a AppHandle,
    api_url: &'a str,
    silent: bool,
}

// ── B2 helpers ───────────────────────────────────────────────────────────────

async fn authorize(client: &Client, key_id: &str, app_key: &str) -> Result<Auth> {
    let resp = client
        .get("https://api.backblazeb2.com/b2api/v3/b2_authorize_account")
        .basic_auth(key_id, Some(app_key))
        .send()
        .await
        .context("B2 authorize request failed")?
        .error_for_status()
        .context("B2 authorize rejected credentials")?
        .json::<AuthResponse>()
        .await
        .context("B2 authorize response parse failed")?;
    Ok(Auth {
        token: resp.authorization_token,
        api_url: resp.api_info.storage_api.api_url,
        download_url: resp.api_info.storage_api.download_url,
    })
}

async fn upload_small(ctx: &UploadCtx<'_>, size: u64) -> Result<String> {
    let UploadCtx { client, auth, bucket_id, file_name, path, app, api_url, silent } = ctx;
    let url_resp: serde_json::Value = client
        .post(format!("{}/b2api/v3/b2_get_upload_url", api_url))
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({ "bucketId": bucket_id }))
        .send()
        .await
        .context("b2_get_upload_url failed")?
        .error_for_status()
        .context("b2_get_upload_url error")?
        .json()
        .await?;
    let upload_url = url_resp["uploadUrl"].as_str().context("missing uploadUrl")?.to_string();
    let upload_token = url_resp["authorizationToken"].as_str().context("missing authorizationToken")?.to_string();

    let bytes = std::fs::read(path).context("Cannot read file")?;
    let sha1 = hex::encode(Sha1::digest(&bytes));

    let resp: UploadResponse = client
        .post(&upload_url)
        .header("Authorization", upload_token)
        .header("X-Bz-File-Name", urlencoding::encode(file_name).as_ref())
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", size.to_string())
        .header("X-Bz-Content-Sha1", &sha1)
        .body(bytes)
        .send()
        .await
        .context("B2 small file upload failed")?
        .error_for_status()
        .context("B2 upload rejected")?
        .json()
        .await
        .context("B2 upload response parse failed")?;

    if !silent {
        let _ = app.emit("cloud:progress", CloudProgress { uploaded: size, total: size, part: 1, parts: 1 });
    }
    Ok(resp.file_id)
}

async fn upload_large(ctx: &UploadCtx<'_>, total_size: u64) -> Result<String> {
    let UploadCtx { client, auth, bucket_id, file_name, path, app, api_url, silent } = ctx;
    let start_resp: StartLargeFileResponse = client
        .post(format!("{}/b2api/v3/b2_start_large_file", api_url))
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({
            "bucketId": bucket_id,
            "fileName": file_name,
            "contentType": "application/octet-stream"
        }))
        .send()
        .await
        .context("b2_start_large_file failed")?
        .error_for_status()
        .context("b2_start_large_file error")?
        .json()
        .await
        .context("b2_start_large_file parse failed")?;
    let file_id = start_resp.file_id;

    let total_parts = total_size.div_ceil(PART_SIZE);
    let mut sha1_array: Vec<String> = Vec::with_capacity(total_parts as usize);
    let mut file = std::fs::File::open(path).context("Cannot open file")?;
    let mut uploaded: u64 = 0;

    for part_num in 1..=total_parts {
        let this_part_size = if part_num == total_parts {
            total_size - (total_parts - 1) * PART_SIZE
        } else {
            PART_SIZE
        };

        let part_url_resp: GetUploadPartUrlResponse = client
            .post(format!("{}/b2api/v3/b2_get_upload_part_url", api_url))
            .bearer_auth(&auth.token)
            .json(&serde_json::json!({ "fileId": file_id }))
            .send()
            .await
            .context("b2_get_upload_part_url failed")?
            .error_for_status()
            .context("b2_get_upload_part_url error")?
            .json()
            .await
            .context("b2_get_upload_part_url parse failed")?;

        let mut buf = vec![0u8; this_part_size as usize];
        file.read_exact(&mut buf).context("Cannot read file chunk")?;
        let sha1 = hex::encode(Sha1::digest(&buf));

        let status = client
            .post(&part_url_resp.upload_url)
            .header("Authorization", &part_url_resp.authorization_token)
            .header("X-Bz-Part-Number", part_num.to_string())
            .header("Content-Length", this_part_size.to_string())
            .header("X-Bz-Content-Sha1", &sha1)
            .body(buf)
            .send()
            .await
            .context("B2 upload_part failed")?
            .error_for_status()
            .context("B2 upload_part error")?
            .status();

        if !status.is_success() {
            bail!("B2 upload_part returned status {}", status);
        }

        sha1_array.push(sha1);
        uploaded += this_part_size;
        if !silent {
            let _ = app.emit(
                "cloud:progress",
                CloudProgress { uploaded, total: total_size, part: part_num, parts: total_parts },
            );
        }
    }

    let finish_resp: UploadResponse = client
        .post(format!("{}/b2api/v3/b2_finish_large_file", api_url))
        .bearer_auth(&auth.token)
        .json(&FinishLargeFileRequest { file_id: &file_id, part_sha1_array: sha1_array })
        .send()
        .await
        .context("b2_finish_large_file failed")?
        .error_for_status()
        .context("b2_finish_large_file error")?
        .json()
        .await
        .context("b2_finish_large_file parse failed")?;

    Ok(finish_resp.file_id)
}

// ── Download (restore path) ──────────────────────────────────────────────────

async fn run_download(backup_id: String) -> Result<String> {
    let client = Client::new();
    let key = fetch_download_key(&client, &backup_id).await?;
    let auth = authorize(&client, &key.key_id, &key.application_key).await?;

    let staging = std::env::temp_dir().join(format!(
        "ferry-cloud-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&staging).context("Cannot create download staging directory")?;

    download_file_by_name(
        &client,
        &auth,
        &key.bucket_name,
        &format!("{}/backup.salt", backup_id),
        &staging.join("backup.salt"),
    )
    .await
    .context("Failed to download backup.salt — check the backup ID")?;

    download_file_by_name(
        &client,
        &auth,
        &key.bucket_name,
        &format!("{}/Backup.enc", backup_id),
        &staging.join("Backup.enc"),
    )
    .await
    .context("Failed to download Backup.enc — check the backup ID")?;

    Ok(staging.to_string_lossy().to_string())
}

async fn download_file_by_name(
    client: &Client,
    auth: &Auth,
    bucket_name: &str,
    file_name: &str,
    dest: &Path,
) -> Result<()> {
    let url = format!(
        "{}/file/{}/{}",
        auth.download_url,
        urlencoding::encode(bucket_name),
        file_name
            .split('/')
            .map(urlencoding::encode)
            .collect::<Vec<_>>()
            .join("/")
    );
    let bytes = client
        .get(&url)
        .header("Authorization", &auth.token)
        .send()
        .await
        .context("B2 download request failed")?
        .error_for_status()
        .context("B2 download rejected")?
        .bytes()
        .await
        .context("B2 download body read failed")?;
    std::fs::write(dest, &bytes).context("Could not write downloaded file")?;
    Ok(())
}
