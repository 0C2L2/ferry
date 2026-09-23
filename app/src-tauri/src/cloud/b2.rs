/// Ferry Cloud Backup — managed, on Ferry's own Backblaze B2 account.
///
/// The desktop app never holds Ferry's B2 master key. The user signs in with
/// an emailed code to Ferry's server (Cloudflare Worker, see ../../../../server),
/// which then hands out a brand-new, disposable B2 Application Key scoped to
/// exactly one backup's folder, one capability (write for upload, read for
/// restore), and a short lifetime. Even a leaked key can't touch any other
/// backup, and only the backup's owner can get one.
///
/// Each backup lives at `<backup_id>/Backup.enc` and `<backup_id>/backup.salt`
/// in Ferry's shared bucket. The server emails the ID to the owner; restoring
/// on a new machine is "sign in → pick the backup". After a successful restore
/// the app asks the server to delete it.
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
// B2 rejects a large file with only ONE part (b2_finish_large_file → 400), so
// anything that would fit in a single part goes single-shot instead (B2 allows
// up to 5 GB that way). Only files over one part size use the large-file API.
const SMALL_FILE_LIMIT: u64 = PART_SIZE;

/// Ferry's server. `FERRY_SERVER_URL` points a dev build at a local copy
/// (`npm run dev` in server/ → http://127.0.0.1:8788).
const DEFAULT_SERVER: &str = "https://api.ferryapp.download";

fn server_url() -> String {
    std::env::var("FERRY_SERVER_URL").unwrap_or_else(|_| DEFAULT_SERVER.to_string())
}

/// The signed-in session for this run of the app. It stays in the Rust process
/// — never handed to the webview — and is gone when the app closes.
#[derive(Clone)]
struct Session {
    token: String,
    /// None when signed in with a restore code rather than an email.
    email: Option<String>,
}

static SESSION: std::sync::Mutex<Option<Session>> = std::sync::Mutex::new(None);

fn session() -> Result<Session> {
    SESSION.lock().unwrap().clone().context("Please sign in to Ferry Cloud first.")
}

/// One call to Ferry's server. Its `{ "error": "…" }` messages are written for
/// users, so they are passed through as the error text.
async fn server<T: serde::de::DeserializeOwned>(
    client: &Client,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
    signed_in: bool,
) -> Result<T> {
    let mut req = client.request(method, format!("{}{}", server_url(), path));
    if signed_in {
        req = req.bearer_auth(session()?.token);
    }
    if let Some(body) = body {
        req = req.json(&body);
    }
    let res = req.send().await.context("Could not reach the Ferry Cloud service")?;
    let status = res.status();
    if status.is_success() {
        return res.json().await.context("Ferry Cloud returned an unexpected response");
    }
    if status == reqwest::StatusCode::UNAUTHORIZED && signed_in {
        *SESSION.lock().unwrap() = None;
    }
    let message = res
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v["error"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("Ferry Cloud error {status}"));
    bail!(message)
}

/// Backup IDs are UUIDs; checking before they go into a URL path keeps a
/// malformed one from addressing some other route.
fn backup_path(backup_id: &str, action: &str) -> Result<String> {
    let ok = backup_id.len() == 36 && backup_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if !ok {
        bail!("That doesn't look like a Ferry backup ID");
    }
    Ok(format!("/api/cloud/backups/{backup_id}/{action}"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatus {
    reachable: bool,
    free: bool,
    /// Email sign-in is switched on (admin settings). Otherwise the restore
    /// code is the only way in.
    email_sign_in: bool,
    signed_in: bool,
    email: Option<String>,
}

/// Is the server up, is cloud free right now, and who is signed in.
#[tauri::command]
pub async fn cloud_status() -> Result<CloudStatus, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let ready = server::<serde_json::Value>(&client, reqwest::Method::GET, "/ready", None, false).await;
    let session = SESSION.lock().unwrap().clone();
    Ok(CloudStatus {
        reachable: ready.is_ok(),
        free: ready.as_ref().map(|v| v["free"] == true).unwrap_or(false),
        email_sign_in: ready.as_ref().map(|v| v["emailSignIn"] == true).unwrap_or(false),
        signed_in: session.is_some(),
        email: session.and_then(|s| s.email),
    })
}

/// Starts a restore-code identity for this upload. Returns the code the user
/// must keep — it is the only way to reach the backup from another computer.
#[tauri::command]
pub async fn cloud_start_anonymous() -> Result<String, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Started {
        token: String,
        restore_code: String,
    }
    let s: Started = server(&Client::new(), reqwest::Method::POST, "/auth/anonymous", None, false)
        .await
        .map_err(|e| format!("{e:#}"))?;
    *SESSION.lock().unwrap() = Some(Session { token: s.token, email: None });
    Ok(s.restore_code)
}

/// Signs in on the new computer with the restore code.
#[tauri::command]
pub async fn cloud_sign_in_code(code: String) -> Result<(), String> {
    #[derive(Deserialize)]
    struct SignedIn {
        token: String,
    }
    let body = serde_json::json!({ "code": code });
    let s: SignedIn = server(&Client::new(), reqwest::Method::POST, "/auth/restore-code", Some(body), false)
        .await
        .map_err(|e| format!("{e:#}"))?;
    *SESSION.lock().unwrap() = Some(Session { token: s.token, email: None });
    Ok(())
}

/// Writes the restore code to a text file the user picked in a save dialog.
#[tauri::command]
pub async fn save_restore_code(path: String, code: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    // The path comes from a save dialog, but the webview passes it on: only
    // ever write a .txt file.
    if path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() != Some("txt") {
        return Err("Save the restore code as a .txt file".into());
    }
    let text = format!(
        "Ferry Cloud restore code\r\n\r\n{code}\r\n\r\n\
         On your new computer: open Ferry → Restore → Restore from Ferry Cloud,\r\n\
         enter this code, then your backup password.\r\n\
         The cloud copy is deleted 30 days after upload.\r\n\
         Keep this file somewhere other than the Ferry USB drive.\r\n"
    );
    std::fs::write(&path, text).map_err(|e| format!("Could not save the file: {e}"))
}

/// Emails a 6-digit sign-in code.
#[tauri::command]
pub async fn cloud_sign_in_start(email: String) -> Result<(), String> {
    let body = serde_json::json!({ "email": email });
    server::<serde_json::Value>(&Client::new(), reqwest::Method::POST, "/auth/start", Some(body), false)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:#}"))
}

/// Trades the emailed code for a session. Returns the signed-in email.
#[tauri::command]
pub async fn cloud_sign_in_verify(email: String, code: String) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Verified {
        token: String,
        email: String,
    }
    let body = serde_json::json!({ "email": email, "code": code });
    let v: Verified = server(&Client::new(), reqwest::Method::POST, "/auth/verify", Some(body), false)
        .await
        .map_err(|e| format!("{e:#}"))?;
    *SESSION.lock().unwrap() = Some(Session { token: v.token, email: Some(v.email.clone()) });
    Ok(v.email)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudBackup {
    id: String,
    tier: String,
    status: String,
    created: i64,
    expires: Option<i64>,
}

/// The signed-in user's cloud backups, newest first.
#[tauri::command]
pub async fn cloud_list_backups() -> Result<Vec<CloudBackup>, String> {
    #[derive(Deserialize)]
    struct List {
        backups: Vec<CloudBackup>,
    }
    server::<List>(&Client::new(), reqwest::Method::GET, "/api/cloud/backups", None, true)
        .await
        .map(|l| l.backups)
        .map_err(|e| format!("{e:#}"))
}

/// Upload `<usb_root>/Backup.enc` + `backup.salt` for the signed-in user.
/// Returns the backup ID (the server also emails it to them).
#[tauri::command]
pub async fn upload_backup_b2(app: AppHandle, usb_root: String) -> Result<String, String> {
    run_upload(app, PathBuf::from(usb_root)).await.map_err(|e| format!("{e:#}"))
}

/// Download one of the signed-in user's backups into a fresh staging
/// directory. Returns that directory's path (pass it to `decrypt_backup`).
#[tauri::command]
pub async fn download_backup_b2(backup_id: String) -> Result<String, String> {
    run_download(backup_id).await.map_err(|e| format!("{e:#}"))
}

/// Deletes a backup from Ferry's cloud storage — call once restore succeeds.
#[tauri::command]
pub async fn delete_cloud_backup(backup_id: String) -> Result<(), String> {
    run_delete(backup_id).await.map_err(|e| format!("{e:#}"))
}

// ── Scoped B2 keys ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopedKey {
    backup_id: String,
    key_id: String,
    application_key: String,
    bucket_id: String,
    bucket_name: String,
}

async fn run_delete(backup_id: String) -> Result<()> {
    let path = backup_path(&backup_id, "delete")?;
    server::<serde_json::Value>(&Client::new(), reqwest::Method::POST, &path, None, true)
        .await
        .context("Ferry Cloud could not delete the backup")?;
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
    let total = enc_path.metadata().context("Cannot read Backup.enc size")?.len()
        + salt_path.metadata().context("Cannot read backup.salt size")?.len();

    // Reserve a backup. Free mode ignores the tier; paid mode prices by it.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Checkout {
        backup_id: String,
        checkout_url: Option<String>,
    }
    const GB: u64 = 1024 * 1024 * 1024;
    let tier = match total {
        t if t <= 50 * GB => "50gb",
        t if t <= 200 * GB => "200gb",
        _ => "1tb",
    };
    let body = serde_json::json!({ "tier": tier, "size": total });
    let checkout: Checkout =
        server(&client, reqwest::Method::POST, "/api/cloud/checkout", Some(body), true).await?;
    if checkout.checkout_url.is_some() {
        // ponytail: paid mode needs "open checkout in the browser, poll until
        // paid"; build it when a payment provider is switched on.
        bail!("Ferry Cloud now asks for payment, which this version of Ferry can't do yet. Please update Ferry.");
    }

    let key: ScopedKey = server(
        &client,
        reqwest::Method::POST,
        &backup_path(&checkout.backup_id, "upload-key")?,
        None,
        true,
    )
    .await?;
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

    // Tells the server the upload is complete: it checks the size, starts the
    // 30-day clock and emails the backup ID to the user.
    server::<serde_json::Value>(
        &client,
        reqwest::Method::POST,
        &backup_path(&key.backup_id, "uploaded")?,
        None,
        true,
    )
    .await
    .context("The upload finished but Ferry Cloud could not confirm it")?;

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
        .header("Authorization", &auth.token) // B2 wants the raw token; "Bearer …" is 401 bad_auth_token
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
        .header("Authorization", &auth.token) // B2 wants the raw token; "Bearer …" is 401 bad_auth_token
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
            .header("Authorization", &auth.token) // B2 wants the raw token; "Bearer …" is 401 bad_auth_token
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
        .header("Authorization", &auth.token) // B2 wants the raw token; "Bearer …" is 401 bad_auth_token
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
    let key: ScopedKey = server(
        &client,
        reqwest::Method::POST,
        &backup_path(&backup_id, "download-key")?,
        None,
        true,
    )
    .await?;
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
