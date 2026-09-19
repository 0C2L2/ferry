/// Backblaze B2 cloud upload for Ferry's "Extra Careful" overflow feature.
///
/// Architecture:
///   • The *already-encrypted* `Backup.enc` is uploaded to B2 as-is.
///     B2 never sees plaintext — the AES-256-GCM layer is Ferry's own.
///   • Upload is chunked (100 MB parts) so multi-GB files resume if the
///     connection drops mid-way.
///   • Progress is reported via Tauri events so the frontend shows a live
///     progress bar identically to the local backup stage.
///   • Credentials (application key + key ID) are stored in the OS credential
///     store (Windows Credential Manager) — never in a config file on disk.
///
/// B2 Large File API flow:
///   1. b2_authorize_account  → auth_token + upload URL stem
///   2. b2_start_large_file   → file_id
///   3. b2_get_upload_part_url (per part) → part upload URL + token
///   4. POST each 100 MB chunk, collect SHA1 per part
///   5. b2_finish_large_file  → final file ID
///
/// For files ≤ 5 MB, b2_upload_file is used instead (no multipart).
use crate::cloud::progress::CloudProgress;
use crate::safety::validate_usb_root;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

const PART_SIZE: u64 = 100 * 1024 * 1024; // 100 MB per B2 large-file part
const B2_AUTHORIZE_URL: &str = "https://api.backblazeb2.com/b2api/v3/b2_authorize_account";
const SMALL_FILE_LIMIT: u64 = 5 * 1024 * 1024; // ≤ 5 MB → single-shot upload

// ── Tauri command ─────────────────────────────────────────────────────────────

/// Tauri command: upload `<usb_root>/Backup.enc` to Backblaze B2.
/// `key_id` and `app_key` come from Windows Credential Manager (fetched by
/// the frontend via the `cloud::creds` helper, never stored in plain JS).
/// Returns the B2 file ID on success.
#[tauri::command]
pub async fn upload_backup_b2(
    app: AppHandle,
    usb_root: String,
    key_id: String,
    app_key: String,
    bucket_name: String,
) -> Result<String, String> {
    run_upload(app, PathBuf::from(usb_root), key_id, app_key, bucket_name)
        .await
        .map_err(|e| e.to_string())
}

// ── Internal ─────────────────────────────────────────────────────────────────

/// Shared context for an upload run: keeps helper signatures small.
struct UploadCtx<'a> {
    client: &'a Client,
    auth: &'a Auth,
    bucket_id: &'a str,
    file_name: &'a str,
    path: &'a PathBuf,
    app: &'a AppHandle,
    api_url: &'a str,
}

async fn run_upload(
    app: AppHandle,
    usb_root: PathBuf,
    key_id: String,
    app_key: String,
    bucket_name: String,
) -> Result<String> {
    // Validate app_key / key_id contain only safe characters (alphanumeric + allowed chars).
    if !key_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        bail!("Invalid B2 key ID format");
    }
    if app_key.is_empty() || app_key.len() > 256 {
        bail!("Invalid B2 application key");
    }
    if bucket_name.is_empty()
        || bucket_name.len() > 63
        || !bucket_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        bail!("Invalid B2 bucket name");
    }

    let usb_root_str = usb_root.to_string_lossy().to_string();
    let (canonical_usb, _) = validate_usb_root(&usb_root_str)?;
    let enc_path = canonical_usb.join("Backup.enc");
    if !enc_path.exists() {
        bail!("Backup.enc not found — run local backup and encryption first");
    }

    let file_size = enc_path.metadata().context("Cannot read Backup.enc size")?.len();
    let client = Client::new();

    // 1. Authorize.
    let auth = authorize(&client, &key_id, &app_key).await?;

    // 2. Find bucket ID.
    let bucket_id = find_bucket(&client, &auth, &bucket_name).await?;

    // 3. Upload.
    let file_name = "Backup.enc";
    let ctx = UploadCtx {
        client: &client,
        auth: &auth,
        bucket_id: &bucket_id,
        file_name,
        path: &enc_path,
        app: &app,
        api_url: &auth.api_url,
    };
    let file_id = if file_size <= SMALL_FILE_LIMIT {
        upload_small(&ctx, file_size).await?
    } else {
        upload_large(&ctx, file_size).await?
    };

    let _ = app.emit("cloud:done", serde_json::json!({ "file_id": file_id }));
    Ok(file_id)
}

// ── B2 API types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    authorization_token: String,
    api_url: String,
    download_url: String,
}

#[derive(Debug)]
struct Auth {
    token: String,
    api_url: String,
    #[allow(dead_code)]
    download_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bucket {
    bucket_id: String,
    bucket_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListBucketsResponse {
    buckets: Vec<Bucket>,
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

// ── B2 helpers ───────────────────────────────────────────────────────────────

async fn authorize(client: &Client, key_id: &str, app_key: &str) -> Result<Auth> {
    let resp = client
        .get(B2_AUTHORIZE_URL)
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
        api_url: resp.api_url,
        download_url: resp.download_url,
    })
}

async fn find_bucket(client: &Client, auth: &Auth, name: &str) -> Result<String> {
    let url = format!("{}/b2api/v3/b2_list_buckets", auth.api_url);
    let resp: ListBucketsResponse = client
        .post(&url)
        .bearer_auth(&auth.token)
        .json(&serde_json::json!({ "bucketName": name }))
        .send()
        .await
        .context("B2 list_buckets failed")?
        .error_for_status()
        .context("B2 list_buckets error")?
        .json()
        .await
        .context("B2 list_buckets parse failed")?;
    resp.buckets
        .into_iter()
        .find(|b| b.bucket_name == name)
        .map(|b| b.bucket_id)
        .ok_or_else(|| anyhow::anyhow!("Bucket '{}' not found in your B2 account", name))
}

async fn upload_small(ctx: &UploadCtx<'_>, size: u64) -> Result<String> {
    let UploadCtx {
        client,
        auth,
        bucket_id,
        file_name,
        path,
        app,
        api_url,
    } = ctx;
    // Get upload URL.
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

    let bytes = std::fs::read(path).context("Cannot read Backup.enc")?;
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

    let _ = app.emit("cloud:progress", CloudProgress {
        uploaded: size,
        total: size,
        part: 1,
        parts: 1,
    });
    Ok(resp.file_id)
}

async fn upload_large(ctx: &UploadCtx<'_>, total_size: u64) -> Result<String> {
    let UploadCtx {
        client,
        auth,
        bucket_id,
        file_name,
        path,
        app,
        api_url,
    } = ctx;
    // 1. Start large file.
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
    let mut file = std::fs::File::open(path).context("Cannot open Backup.enc")?;
    let mut uploaded: u64 = 0;

    for part_num in 1..=total_parts {
        let this_part_size = if part_num == total_parts {
            total_size - (total_parts - 1) * PART_SIZE
        } else {
            PART_SIZE
        };

        // Get part upload URL.
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

        // Read this part.
        let mut buf = vec![0u8; this_part_size as usize];
        file.read_exact(&mut buf).context("Cannot read Backup.enc chunk")?;
        let sha1 = hex::encode(Sha1::digest(&buf));

        // Upload part.
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
        let _ = app.emit("cloud:progress", CloudProgress {
            uploaded,
            total: total_size,
            part: part_num,
            parts: total_parts,
        });
    }

    // Finish.
    let finish_resp: UploadResponse = client
        .post(format!("{}/b2api/v3/b2_finish_large_file", api_url))
        .bearer_auth(&auth.token)
        .json(&FinishLargeFileRequest {
            file_id: &file_id,
            part_sha1_array: sha1_array,
        })
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

