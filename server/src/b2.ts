// Ferry's own Backblaze B2 account, used server-side ONLY. The master key
// never reaches the desktop app. Instead, for every backup we mint a
// brand-new, disposable Application Key scoped to:
//   - this one bucket
//   - a namePrefix unique to that backup (`<backupId>/`)
//   - a single capability (writeFiles for upload, readFiles for restore)
//   - a short validity window
// Even if a minted key leaked, it could only touch that one backup's files,
// with that one capability, for a few hours at most.

export interface B2Env {
  B2_MASTER_KEY_ID: string;
  B2_MASTER_APPLICATION_KEY: string;
  B2_BUCKET_NAME: string;
}

const B2_API_BASE = "https://api.backblazeb2.com/b2api/v3";

interface MasterAuth {
  authToken: string;
  apiUrl: string;
  accountId: string;
  bucketId: string;
}

// Lives as long as the Worker isolate: a best-effort saving of one round
// trip, never relied on for correctness.
let cached: { auth: MasterAuth; expiresAt: number } | null = null;

async function master(env: B2Env): Promise<MasterAuth> {
  if (cached && cached.expiresAt > Date.now()) return cached.auth;
  if (!env.B2_MASTER_KEY_ID || !env.B2_MASTER_APPLICATION_KEY || !env.B2_BUCKET_NAME) {
    throw new Error("B2 is not configured (B2_MASTER_KEY_ID / B2_MASTER_APPLICATION_KEY / B2_BUCKET_NAME)");
  }
  const res = await fetch(`${B2_API_BASE}/b2_authorize_account`, {
    headers: { Authorization: "Basic " + btoa(`${env.B2_MASTER_KEY_ID}:${env.B2_MASTER_APPLICATION_KEY}`) },
  });
  if (!res.ok) throw new Error(`B2 master authorize failed: ${res.status}`);
  // v3 nests apiUrl under apiInfo.storageApi — verified against a live call.
  const data = (await res.json()) as {
    authorizationToken: string;
    accountId: string;
    apiInfo: { storageApi: { apiUrl: string } };
  };
  const apiUrl = data.apiInfo.storageApi.apiUrl;

  const buckets = (await b2Post(apiUrl, data.authorizationToken, "b2_list_buckets", {
    accountId: data.accountId,
    bucketName: env.B2_BUCKET_NAME,
  })) as { buckets: { bucketId: string; bucketName: string }[] };
  const bucket = buckets.buckets.find(b => b.bucketName === env.B2_BUCKET_NAME);
  if (!bucket) throw new Error(`Bucket '${env.B2_BUCKET_NAME}' not found in this B2 account`);

  const auth = { authToken: data.authorizationToken, apiUrl, accountId: data.accountId, bucketId: bucket.bucketId };
  cached = { auth, expiresAt: Date.now() + 12 * 60 * 60 * 1000 }; // tokens last ~24h
  return auth;
}

async function b2Post(apiUrl: string, token: string, op: string, body: unknown): Promise<unknown> {
  const res = await fetch(`${apiUrl}/b2api/v3/${op}`, {
    method: "POST",
    headers: { Authorization: token, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`B2 ${op} failed: ${res.status} ${await res.text()}`);
  return res.json();
}

export interface ScopedKey {
  backupId: string;
  keyId: string;
  applicationKey: string;
  bucketId: string;
  bucketName: string;
}

async function scopedKey(
  env: B2Env,
  capability: "writeFiles" | "readFiles",
  backupId: string,
  seconds: number,
): Promise<ScopedKey> {
  const auth = await master(env);
  const data = (await b2Post(auth.apiUrl, auth.authToken, "b2_create_key", {
    accountId: auth.accountId,
    capabilities: [capability],
    keyName: `ferry-${capability === "writeFiles" ? "up" : "dl"}-${backupId}`.slice(0, 100),
    bucketId: auth.bucketId,
    namePrefix: `${backupId}/`,
    validDurationInSeconds: seconds,
  })) as { applicationKeyId: string; applicationKey: string };
  return {
    backupId,
    keyId: data.applicationKeyId,
    applicationKey: data.applicationKey,
    bucketId: auth.bucketId,
    bucketName: env.B2_BUCKET_NAME,
  };
}

/** Upload-only, 6 hours — generous for a large backup on a slow line. */
export const createUploadKey = (env: B2Env, backupId: string) => scopedKey(env, "writeFiles", backupId, 6 * 3600);

/** Read-only, 30 minutes — plenty for one download. */
export const createDownloadKey = (env: B2Env, backupId: string) => scopedKey(env, "readFiles", backupId, 30 * 60);

/** Total bytes stored under `<backupId>/` (finished files only). */
export async function backupSize(env: B2Env, backupId: string): Promise<number> {
  const auth = await master(env);
  const data = (await b2Post(auth.apiUrl, auth.authToken, "b2_list_file_names", {
    bucketId: auth.bucketId,
    prefix: `${backupId}/`,
    maxFileCount: 100,
  })) as { files: { contentLength: number }[] };
  return data.files.reduce((sum, f) => sum + f.contentLength, 0);
}

/** Removes every file version AND any unfinished large upload under
 *  `<backupId>/` — B2 bills half-uploaded parts as storage too. */
export async function deleteBackup(env: B2Env, backupId: string): Promise<void> {
  const auth = await master(env);
  const prefix = `${backupId}/`;

  const unfinished = (await b2Post(auth.apiUrl, auth.authToken, "b2_list_unfinished_large_files", {
    bucketId: auth.bucketId,
    namePrefix: prefix,
  })) as { files: { fileId: string }[] };
  for (const f of unfinished.files) {
    await b2Post(auth.apiUrl, auth.authToken, "b2_cancel_large_file", { fileId: f.fileId });
  }

  const versions = (await b2Post(auth.apiUrl, auth.authToken, "b2_list_file_versions", {
    bucketId: auth.bucketId,
    prefix,
    maxFileCount: 100,
  })) as { files: { fileId: string; fileName: string }[] };
  for (const f of versions.files) {
    await b2Post(auth.apiUrl, auth.authToken, "b2_delete_file_version", { fileId: f.fileId, fileName: f.fileName });
  }
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isValidBackupId(id: unknown): id is string {
  return typeof id === "string" && UUID_RE.test(id);
}
