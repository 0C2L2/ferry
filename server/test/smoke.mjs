// End-to-end check against a running local server (`npm run dev`, DEV_MODE=1,
// ADMIN_TOKEN and real B2 credentials in .dev.vars). Walks the whole Cloud
// Backup life with a restore code — start → reserve → scoped key → upload to
// B2 → confirm → sign in again with the code → download key → delete — plus
// the refusals, the admin settings, the abuse limits, and email sign-in
// switched on and off from the admin API.
//
//   npm run dev        # terminal 1
//   npm run smoke      # terminal 2
import { createHash, createHmac } from "node:crypto";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

// Local runs share one IP, so earlier runs' rate-limit windows would leak into
// this one: start from a clean slate.
spawnSync("npx", ["wrangler", "d1", "execute", "ferry", "--local", "--command", '"DELETE FROM rate_limits"'], {
  cwd: new URL("..", import.meta.url),
  shell: true,
  stdio: "ignore",
});

const BASE = process.env.FERRY_SERVER ?? "http://127.0.0.1:8788";
const vars = Object.fromEntries(
  readFileSync(new URL("../.dev.vars", import.meta.url), "utf8")
    .split(/\r?\n/)
    .filter(l => l.includes("=") && !l.startsWith("#"))
    .map(l => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1)]),
);
const ADMIN = vars.ADMIN_TOKEN;

let passed = 0;
function check(cond, what) {
  if (!cond) throw new Error(`FAILED: ${what}`);
  passed++;
  console.log(`  ok  ${what}`);
}

async function call(method, path, { token, body, headers = {} } = {}) {
  const res = await fetch(BASE + path, {
    method,
    headers: {
      "content-type": "application/json",
      ...(token ? { authorization: `Bearer ${token}` } : {}),
      ...headers,
    },
    body: typeof body === "string" ? body : body && JSON.stringify(body),
  });
  return { status: res.status, body: await res.json().catch(() => null) };
}

const settings = patch => call("POST", "/admin/settings", { token: ADMIN, body: patch });

async function anonymous() {
  const r = await call("POST", "/auth/anonymous");
  check(r.status === 200 && /^FERRY(-[2-9A-HJ-NP-Z]{4}){4}$/.test(r.body.restoreCode), "new restore code issued");
  return r.body;
}

// Uploads one small file with a scoped key, exactly as the app does.
async function uploadWith(key, name, bytes) {
  const auth = await (
    await fetch("https://api.backblazeb2.com/b2api/v3/b2_authorize_account", {
      headers: { Authorization: "Basic " + Buffer.from(`${key.keyId}:${key.applicationKey}`).toString("base64") },
    })
  ).json();
  const up = await (
    await fetch(`${auth.apiInfo.storageApi.apiUrl}/b2api/v3/b2_get_upload_url`, {
      method: "POST",
      headers: { Authorization: auth.authorizationToken },
      body: JSON.stringify({ bucketId: key.bucketId }),
    })
  ).json();
  const res = await fetch(up.uploadUrl, {
    method: "POST",
    headers: {
      Authorization: up.authorizationToken,
      "X-Bz-File-Name": encodeURIComponent(`${key.backupId}/${name}`),
      "Content-Type": "application/octet-stream",
      "X-Bz-Content-Sha1": createHash("sha1").update(bytes).digest("hex"),
    },
    body: bytes,
  });
  return res.status;
}

console.log(`Smoke test against ${BASE}`);

// ── Admin + settings ─────────────────────────────────────────────────────────
check((await call("GET", "/admin/overview")).status === 401, "admin refuses without a token");
check((await call("GET", "/admin/overview", { token: "wrong" })).status === 401, "admin refuses a wrong token");
check((await settings({ emailEnabled: false, cloudFree: true, maxActiveBackups: 50, perIpDaily: 100 })).status === 200, "admin saves settings");
const ready = await call("GET", "/ready");
check(ready.status === 200 && ready.body.free === true && ready.body.emailSignIn === false, "/ready reports free + email off");
check((await call("POST", "/auth/start", { body: { email: "a@example.com" } })).status === 503, "email sign-in refused while off");
check((await call("GET", "/api/cloud/backups")).status === 401, "signed-out requests are refused");

// Contact form (no Resend key locally, so a valid message stops at 503).
const note = { email: "a@example.com", message: "Hello — a question about restoring." };
check((await call("POST", "/contact", { body: { ...note, email: "nope" } })).status === 400, "contact refuses a bad email");
check((await call("POST", "/contact", { body: { ...note, message: "hi" } })).status === 400, "contact refuses a too-short message");
check((await call("POST", "/contact", { body: { ...note, website: "spam.example" } })).body.sent === true, "contact quietly drops bot submissions");
check((await call("POST", "/contact", { body: note })).status === 503, "contact waits for a support inbox + Resend key");

// ── Restore-code life cycle ─────────────────────────────────────────────────
const alice = await anonymous();
const checkout = await call("POST", "/api/cloud/checkout", { token: alice.token, body: { size: 10 } });
check(checkout.status === 200 && checkout.body.backupId && checkout.body.checkoutUrl === null, "free backup reserved");
const id = checkout.body.backupId;
check((await call("POST", "/api/cloud/checkout", { token: alice.token, body: {} })).status === 409, "a second active backup is refused");
check((await call("POST", `/api/cloud/backups/${id}/uploaded`, { token: alice.token })).status === 409, "can't confirm an empty upload");

const bob = await anonymous();
check((await call("GET", `/api/cloud/backups/${id}`, { token: bob.token })).status === 404, "another user can't see it");
check((await call("POST", `/api/cloud/backups/${id}/upload-key`, { token: bob.token })).status === 404, "another user can't upload to it");
const tooBig = await call("POST", "/api/cloud/checkout", { token: bob.token, body: { size: 60 * 1024 ** 3 } });
check(tooBig.status === 413, "a backup over 50 GB is refused before upload");

const key = await call("POST", `/api/cloud/backups/${id}/upload-key`, { token: alice.token });
check(key.status === 200 && key.body.keyId && key.body.backupId === id, "scoped upload key issued");
check((await uploadWith(key.body, "backup.salt", Buffer.from("smoke-test"))) === 200, "scoped key uploads to B2");
const done = await call("POST", `/api/cloud/backups/${id}/uploaded`, { token: alice.token });
check(done.status === 200 && done.body.status === "uploaded" && done.body.expires > Date.now(), "upload confirmed, expiry set");
check((await call("POST", `/api/cloud/backups/${id}/upload-key`, { token: alice.token })).status === 409, "no second upload key");

// On the new PC: only the restore code.
check((await call("POST", "/auth/restore-code", { body: { code: "FERRY-2222-2222-2222-2222" } })).status === 401, "unknown restore code refused");
const again = await call("POST", "/auth/restore-code", { body: { code: alice.restoreCode.toLowerCase().replace(/-/g, " ") } });
check(again.status === 200 && again.body.token, "restore code signs in (case/spacing forgiven)");
const list = await call("GET", "/api/cloud/backups", { token: again.body.token });
check(list.body.backups.some(b => b.id === id && b.status === "uploaded"), "backup found via restore code");
check((await call("POST", `/api/cloud/backups/${id}/download-key`, { token: again.body.token })).status === 200, "download key issued");

// ── Payments webhook (dormant while free) ───────────────────────────────────
const payload = JSON.stringify({
  meta: { event_name: "order_created", custom_data: { backup_id: id } },
  data: { id: "1", attributes: { status: "paid", store_id: Number(vars.LEMONSQUEEZY_STORE_ID), first_order_item: { variant_id: Number(vars.LS_VARIANT_50GB) } } },
});
check((await call("POST", "/webhooks/lemonsqueezy", { body: payload, headers: { "x-signature": "0".repeat(64) } })).status === 401, "forged webhook refused");
const sig = createHmac("sha256", vars.LEMONSQUEEZY_WEBHOOK_SECRET).update(payload).digest("hex");
check((await call("POST", "/webhooks/lemonsqueezy", { body: payload, headers: { "x-signature": sig } })).status === 200, "signed webhook accepted");

// ── Admin view + delete ─────────────────────────────────────────────────────
const overview = await call("GET", "/admin/overview", { token: ADMIN });
check(overview.status === 200 && overview.body.backups.some(b => b.id === id && b.owner === "restore code"), "admin sees the backup");
check(overview.body.settings.resendApiKey === undefined, "admin overview never returns the Resend key");

check((await call("POST", `/api/cloud/backups/${id}/delete`, { token: again.body.token })).status === 200, "owner deletes it");
check((await call("GET", `/api/cloud/backups/${id}`, { token: alice.token })).body.status === "deleted", "backup marked deleted");
check((await call("POST", `/api/cloud/backups/${id}/download-key`, { token: alice.token })).status === 409, "deleted backup can't be downloaded");

const carol = await anonymous();
const c = await call("POST", "/api/cloud/checkout", { token: carol.token, body: {} });
check((await call("POST", `/admin/backups/${c.body.backupId}/delete`, { token: ADMIN })).status === 200, "admin can delete any backup");

// ── Abuse limits ────────────────────────────────────────────────────────────
await settings({ maxActiveBackups: 0 });
const dave = await anonymous();
check((await call("POST", "/api/cloud/checkout", { token: dave.token, body: {} })).status === 503, "global cap refuses when full");
await settings({ maxActiveBackups: 50, perIpDaily: 0 });
check((await call("POST", "/api/cloud/checkout", { token: dave.token, body: {} })).status === 429, "per-network daily limit refuses");

// ── Email sign-in, switched on from admin ───────────────────────────────────
check((await call("POST", "/admin/email-test", { token: ADMIN, body: { to: "x@example.com" } })).status === 400, "email test needs a key first");
await settings({ emailEnabled: true });
check((await call("GET", "/ready")).body.emailSignIn === true, "/ready reports email on");
const email = `smoke-${Date.now()}@example.com`;
const start = await call("POST", "/auth/start", { body: { email } });
check(start.status === 200 && /^\d{6}$/.test(start.body.devCode), "email code issued when email is on");
check((await call("POST", "/auth/verify", { body: { email, code: start.body.devCode === "000000" ? "111111" : "000000" } })).status === 401, "wrong email code refused");
const v = await call("POST", "/auth/verify", { body: { email, code: start.body.devCode } });
check(v.status === 200 && (await call("GET", "/api/me", { token: v.body.token })).body.email === email, "email sign-in works");

// ── Accounts: admin by email, linking a restore code, deleting everything ──
const mine = v.body.token;
check((await call("GET", "/admin/overview", { token: mine })).status === 401, "a normal account is not an admin");
await settings({ adminEmails: `${email}, not-an-email` });
check((await call("GET", "/api/me", { token: mine })).body.isAdmin === true, "listed email is an admin");
check((await call("GET", "/admin/overview", { token: mine })).status === 200, "admin page works with an email session");

await settings({ perIpDaily: 100 });
const eve = await anonymous();
const eveBackup = (await call("POST", "/api/cloud/checkout", { token: eve.token, body: {} })).body.backupId;
check((await call("POST", "/api/account/claim", { token: eve.token, body: { code: eve.restoreCode } })).status === 400, "a restore-code session can't claim");
const claim = await call("POST", "/api/account/claim", { token: mine, body: { code: eve.restoreCode } });
check(claim.status === 200 && claim.body.linked === 1, "restore code linked to the email account");
check((await call("GET", "/api/cloud/backups", { token: mine })).body.backups.some(b => b.id === eveBackup), "linked backup shows in the account");
check((await call("POST", "/api/account/claim", { token: mine, body: { code: eve.restoreCode } })).body.linked === 0, "linking twice is harmless");
const viaCode = await call("POST", "/auth/restore-code", { body: { code: eve.restoreCode } });
check((await call("GET", "/api/me", { token: viaCode.body.token })).body.email === email, "the restore code now opens the email account");

check((await call("POST", "/api/account/delete", { token: mine })).status === 200, "account deleted");
check((await call("GET", "/api/cloud/backups", { token: mine })).status === 401, "its sessions are gone");
check((await call("POST", "/auth/restore-code", { body: { code: eve.restoreCode } })).status === 401, "its restore codes are gone");

// ── Rate limit on sending codes ──────────────────────────────────────────────
let limited = false;
for (let i = 0; i < 12 && !limited; i++) {
  limited = (await call("POST", "/auth/start", { body: { email: `burst-${i}-${Date.now()}@example.com` } })).status === 429;
}
check(limited, "sending codes is rate-limited per network");

await settings({ emailEnabled: false, maxActiveBackups: 50, perIpDaily: 3, adminEmails: "" });
check((await call("GET", "/ready")).body.emailSignIn === false, "settings restored");

console.log(`\nAll ${passed} checks passed.`);
