import { randomUUID } from "node:crypto";

// Ferry's own Backblaze B2 account, used server-side ONLY. The master key
// never reaches the desktop app. Instead, for every backup we mint a
// brand-new, disposable Application Key scoped to:
//   - this one bucket
//   - a namePrefix unique to that backup (`<backupId>/`)
//   - a single capability (writeFiles for upload, readFiles for restore)
//   - a short validity window
// Even if a minted key leaked, it could only touch that one backup's files,
// with that one capability, for a few hours at most.
const B2_API_BASE = "https://api.backblazeb2.com/b2api/v3";

interface MasterAuth {
  authToken: string;
  apiUrl: string;
  accountId: string;
}

let cachedAuth: { auth: MasterAuth; expiresAt: number } | null = null;
let cachedBucketId: string | null = null;

async function authorizeMaster(): Promise<MasterAuth> {
  if (cachedAuth && cachedAuth.expiresAt > Date.now()) return cachedAuth.auth;

  const keyId = process.env.B2_MASTER_KEY_ID;
  const appKey = process.env.B2_MASTER_APPLICATION_KEY;
  if (!keyId || !appKey) {
    throw new Error("B2_MASTER_KEY_ID / B2_MASTER_APPLICATION_KEY are not configured");
  }

  const res = await fetch(`${B2_API_BASE}/b2_authorize_account`, {
    headers: { Authorization: "Basic " + Buffer.from(`${keyId}:${appKey}`).toString("base64") },
  });
  if (!res.ok) throw new Error(`B2 master authorize failed: ${res.status}`);
  // b2_authorize_account (v3) nests apiUrl/downloadUrl under apiInfo.storageApi,
  // not at the top level — verified against a real live call, not assumed.
  const data = (await res.json()) as {
    authorizationToken: string;
    accountId: string;
    apiInfo: { storageApi: { apiUrl: string; downloadUrl: string } };
  };
  const auth: MasterAuth = {
    authToken: data.authorizationToken,
    apiUrl: data.apiInfo.storageApi.apiUrl,
    accountId: data.accountId,
  };
  // B2 auth tokens are valid ~24h; refresh a bit early to be safe.
  cachedAuth = { auth, expiresAt: Date.now() + 12 * 60 * 60 * 1000 };
  return auth;
}

async function resolveBucketId(): Promise<{ bucketId: string; bucketName: string }> {
  const bucketName = process.env.B2_BUCKET_NAME;
  if (!bucketName) throw new Error("B2_BUCKET_NAME is not configured");
  if (cachedBucketId) return { bucketId: cachedBucketId, bucketName };

  const auth = await authorizeMaster();
  const res = await fetch(`${auth.apiUrl}/b2api/v3/b2_list_buckets`, {
    method: "POST",
    headers: { Authorization: auth.authToken, "content-type": "application/json" },
    body: JSON.stringify({ accountId: auth.accountId, bucketName }),
  });
  if (!res.ok) throw new Error(`B2 list_buckets failed: ${res.status}`);
  const data = (await res.json()) as { buckets: { bucketId: string; bucketName: string }[] };
  const bucket = data.buckets.find(b => b.bucketName === bucketName);
  if (!bucket) throw new Error(`Bucket '${bucketName}' not found in this B2 account`);
  cachedBucketId = bucket.bucketId;
  return { bucketId: bucket.bucketId, bucketName };
}

export interface ScopedKey {
  backupId: string;
  keyId: string;
  applicationKey: string;
  bucketId: string;
  bucketName: string;
}

async function createScopedKey(
  capability: "writeFiles" | "readFiles",
  backupId: string,
  validDurationSeconds: number,
): Promise<ScopedKey> {
  const auth = await authorizeMaster();
  const { bucketId, bucketName } = await resolveBucketId();
  const namePrefix = `${backupId}/`;

  const res = await fetch(`${auth.apiUrl}/b2api/v3/b2_create_key`, {
    method: "POST",
    headers: { Authorization: auth.authToken, "content-type": "application/json" },
    body: JSON.stringify({
      accountId: auth.accountId,
      capabilities: [capability],
      keyName: `ferry-${capability === "writeFiles" ? "up" : "dl"}-${backupId}`.slice(0, 100),
      bucketId,
      namePrefix,
      validDurationInSeconds: validDurationSeconds,
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`B2 create_key failed: ${res.status} ${body}`);
  }
  const data = (await res.json()) as { applicationKeyId: string; applicationKey: string };
  return { backupId, keyId: data.applicationKeyId, applicationKey: data.applicationKey, bucketId, bucketName };
}

/** Mints a disposable upload-only key for a brand-new backup. */
export async function createUploadKey(): Promise<ScopedKey> {
  const backupId = randomUUID();
  return createScopedKey("writeFiles", backupId, 6 * 60 * 60); // 6 hours — generous for large uploads
}

/** Mints a disposable read-only key for restoring an existing backup. */
export async function createDownloadKey(backupId: string): Promise<ScopedKey> {
  return createScopedKey("readFiles", backupId, 30 * 60); // 30 minutes is plenty for a download
}

/** Deletes every file version under `<backupId>/` — called once restore succeeds. */
export async function deleteBackup(backupId: string): Promise<void> {
  const auth = await authorizeMaster();
  const { bucketId } = await resolveBucketId();
  const prefix = `${backupId}/`;

  const res = await fetch(`${auth.apiUrl}/b2api/v3/b2_list_file_versions`, {
    method: "POST",
    headers: { Authorization: auth.authToken, "content-type": "application/json" },
    body: JSON.stringify({ bucketId, prefix, maxFileCount: 1000 }),
  });
  if (!res.ok) throw new Error(`B2 list_file_versions failed: ${res.status}`);
  const data = (await res.json()) as { files: { fileId: string; fileName: string }[] };

  for (const file of data.files) {
    const del = await fetch(`${auth.apiUrl}/b2api/v3/b2_delete_file_version`, {
      method: "POST",
      headers: { Authorization: auth.authToken, "content-type": "application/json" },
      body: JSON.stringify({ fileId: file.fileId, fileName: file.fileName }),
    });
    if (!del.ok) throw new Error(`B2 delete_file_version failed for ${file.fileName}: ${del.status}`);
  }
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isValidBackupId(id: unknown): id is string {
  return typeof id === "string" && UUID_RE.test(id);
}
