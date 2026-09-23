// Ferry server (Cloudflare Worker). The desktop app and the website call it
// for sign-in, Cloud Backup bookkeeping, and short-lived B2 keys. It never
// sees backup contents — those are encrypted on the user's PC and go straight
// to B2 with a key scoped to that one backup.
//
//   GET  /ready                                   {free, emailSignIn} (app + site)
//   POST /auth/anonymous                          → {token, restoreCode}
//   POST /auth/restore-code {code}                → {token}
//   POST /auth/start {email}                      email a 6-digit code (if email is on)
//   POST /auth/verify {email, code}               → {token}
//   POST /auth/signout
//   POST /contact {email, topic, message}          contact form → support inbox (Reply-To: sender)
//   GET  /api/me                                  → {email, isAdmin}
//   POST /api/account/claim {code}                link a restore-code backup to your email
//   POST /api/account/delete                      delete every backup + all sign-in data
//   GET  /api/cloud/backups                       your cloud backups
//   POST /api/cloud/checkout {tier, size}         → {backupId, checkoutUrl}
//   GET  /api/cloud/backups/:id                   status
//   POST /api/cloud/backups/:id/upload-key        paid → scoped write key
//   POST /api/cloud/backups/:id/uploaded          upload finished → retention starts
//   POST /api/cloud/backups/:id/download-key      uploaded → scoped read key
//   POST /api/cloud/backups/:id/delete            delete now
//   POST /webhooks/lemonsqueezy                   signed payment notification
//   /admin/*                                      ADMIN_TOKEN, or a session whose email is an admin

import { backupSize, createDownloadKey, createUploadKey, deleteBackup, isValidBackupId, type B2Env } from "./b2";
import {
  normalizeEmail,
  sessionEmail,
  sha256,
  restoreCodeOwner,
  signInWithRestoreCode,
  signOut,
  startAnonymous,
  startLogin,
  verifyLogin,
  type AuthEnv,
} from "./auth";
import { sendEmail } from "./email";
import { createCheckout, isTier, TIERS, variantFor, verifyWebhook, type LemonEnv } from "./lemon";
import { loadSettings, saveSettings, type Settings, type SettingsEnv } from "./settings";

export interface Env extends B2Env, AuthEnv, LemonEnv, SettingsEnv {
  /** Bearer token for the /admin routes (a wrangler secret). */
  ADMIN_TOKEN?: string;
  /** "1" = local development: email codes are returned in the response and
   *  checkouts are marked paid without Lemon Squeezy. Only honoured on
   *  localhost, so a misconfigured deploy cannot turn it on. */
  DEV_MODE?: string;
}

interface BackupRow {
  id: string;
  email: string;
  tier: string;
  status: "unpaid" | "paid" | "uploaded" | "deleted" | "rejected";
  created: number;
  expires: number | null;
  reminded: number;
}

class HttpError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

// Bearer tokens, not cookies, so allowing any origin is safe — and needed:
// the app's webview and the website both call this from a browser context.
const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-headers": "authorization, content-type",
  "access-control-allow-methods": "GET, POST, OPTIONS",
};

const HOUR = 3_600_000;

/** The contact page's topic choices (site/contact.html); anything else is filed as the last. */
const CONTACT_TOPICS = ["A question", "Something went wrong", "Cloud copy or account", "Something else"];

/** Fixed-window rate limit: true while `key` has been hit at most `limit`
 *  times in the current window. One statement, so concurrent hits can't race. */
async function allow(env: Env, key: string, limit: number, windowMs = HOUR): Promise<boolean> {
  const now = Date.now();
  const row = await env.DB.prepare(
    `INSERT INTO rate_limits (key, window_start, count) VALUES (?1, ?2, 1)
     ON CONFLICT(key) DO UPDATE SET
       count = CASE WHEN window_start < ?3 THEN 1 ELSE count + 1 END,
       window_start = CASE WHEN window_start < ?3 THEN ?2 ELSE window_start END
     RETURNING count`,
  )
    .bind(key, now, now - windowMs)
    .first<{ count: number }>();
  return (row?.count ?? 0) <= limit;
}

const tooMany = () => new HttpError(429, "Too many attempts from this network. Please wait a while and try again.");

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json", ...CORS } });

export default {
  async fetch(req, env) {
    if (req.method === "OPTIONS") return new Response(null, { headers: CORS });
    try {
      return await route(req, env);
    } catch (e) {
      if (e instanceof HttpError) return json({ error: e.message }, e.status);
      console.error(e);
      return json({ error: "Something went wrong on Ferry's side. Please try again." }, 500);
    }
  },
  async scheduled(_event, env, ctx) {
    ctx.waitUntil(sweep(env));
  },
} satisfies ExportedHandler<Env>;

async function body(req: Request): Promise<Record<string, unknown>> {
  try {
    const parsed = await req.json();
    return parsed && typeof parsed === "object" ? (parsed as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

async function route(req: Request, env: Env): Promise<Response> {
  const url = new URL(req.url);
  const path = url.pathname;
  const post = req.method === "POST";
  const get = req.method === "GET";
  const dev = env.DEV_MODE === "1" && ["localhost", "127.0.0.1"].includes(url.hostname);
  const s = await loadSettings(env);
  const ip = await sha256(req.headers.get("cf-connecting-ip") ?? "unknown");

  if (get && path === "/ready") return json({ ready: true, free: s.cloudFree, emailSignIn: s.emailEnabled });

  // ── Sign-in ───────────────────────────────────────────────────────────────
  if (post && path === "/auth/anonymous") {
    if (!(await allow(env, `anon:${ip}`, 20))) throw tooMany();
    return json(await startAnonymous(env));
  }

  if (post && path === "/auth/restore-code") {
    if (!(await allow(env, `code:${ip}`, 30))) throw tooMany();
    const token = await signInWithRestoreCode(env, (await body(req)).code);
    if (!token) throw new HttpError(401, "That restore code wasn't recognised. Check it and try again.");
    return json({ token });
  }

  if (post && (path === "/auth/start" || path === "/auth/verify") && !s.emailEnabled) {
    throw new HttpError(503, "Email sign-in is turned off. Use your restore code instead.");
  }
  if (post && path === "/auth/start") {
    const email = normalizeEmail((await body(req)).email);
    if (!email) throw new HttpError(400, "Enter a valid email address.");
    // Each code costs a real email (Resend's free tier: 100/day).
    if (!(await allow(env, `start:${ip}`, 10))) throw tooMany();
    const sent = await startLogin(env, s, email).catch(e => {
      console.error("sign-in email failed", e);
      throw new HttpError(502, "We couldn't send a code to that address. Check it and try again.");
    });
    if (!sent.ok) throw new HttpError(429, "A code was just sent. Wait a minute before asking for another.");
    return json(dev ? { sent: true, devCode: sent.code } : { sent: true });
  }
  if (post && path === "/auth/verify") {
    if (!(await allow(env, `verify:${ip}`, 30))) throw tooMany();
    const b = await body(req);
    const email = normalizeEmail(b.email);
    const token = email && (await verifyLogin(env, email, b.code));
    if (!token) throw new HttpError(401, "That code is wrong or has expired. Ask for a new one.");
    return json({ token, email });
  }

  // ── Contact form → the support inbox, with Reply-To set to the visitor ────
  if (post && path === "/contact") {
    const b = await body(req);
    // Bots fill the hidden "website" field: tell them it worked, send nothing.
    if (b.website) return json({ sent: true });
    const from = normalizeEmail(b.email);
    const topic = CONTACT_TOPICS.find(t => t === b.topic) ?? "Something else";
    const text = typeof b.message === "string" ? b.message.trim() : "";
    if (!from) throw new HttpError(400, "Enter a valid email address so we can reply.");
    if (text.length < 10) throw new HttpError(400, "Write a little more — at least a sentence.");
    if (text.length > 5000) throw new HttpError(400, "That's too long — please keep it under 5,000 characters.");
    if (!s.emailReplyTo || !s.resendApiKey) throw new HttpError(503, "The contact form isn't set up yet.");
    if (!(await allow(env, `contact:${ip}`, 5))) throw tooMany();
    await sendEmail({ ...s, emailReplyTo: from }, s.emailReplyTo, `Contact: ${topic}`, `From: ${from}\nTopic: ${topic}\n\n${text}`, true).catch(
      e => {
        console.error("contact email failed", e);
        throw new HttpError(502, "Your message couldn't be sent just now. Please try again in a few minutes.");
      },
    );
    return json({ sent: true });
  }

  if (post && path === "/webhooks/lemonsqueezy") return lemonWebhook(req, env);
  if (path.startsWith("/admin/")) return admin(req, env, s, path);

  // ── Everything below needs a signed-in user ──────────────────────────────
  const email = await sessionEmail(env, req);
  if (!email) throw new HttpError(401, "Please sign in again.");

  if (post && path === "/auth/signout") {
    await signOut(env, req);
    return json({ ok: true });
  }
  if (get && path === "/api/me") {
    const isEmail = !email.startsWith("anon:");
    return json({ email: isEmail ? email : null, isAdmin: isEmail && s.adminEmails.includes(email) });
  }

  if (post && path === "/api/account/claim") {
    if (email.startsWith("anon:")) throw new HttpError(400, "Sign in with your email first.");
    const code = (await body(req)).code;
    const owner = await restoreCodeOwner(env, code);
    if (!owner) throw new HttpError(404, "That restore code wasn't recognised. Check it and try again.");
    if (owner === email) return json({ linked: 0 });
    if (!owner.startsWith("anon:")) throw new HttpError(409, "That restore code already belongs to another account.");
    const moved = await env.DB.prepare("UPDATE backups SET email = ? WHERE email = ?").bind(email, owner).run();
    await env.DB.batch([
      env.DB.prepare("UPDATE restore_codes SET identity = ? WHERE identity = ?").bind(email, owner),
      env.DB.prepare("DELETE FROM sessions WHERE email = ?").bind(owner),
    ]);
    return json({ linked: moved.meta.changes ?? 0 });
  }

  if (post && path === "/api/account/delete") {
    const { results } = await env.DB.prepare(
      "SELECT * FROM backups WHERE email = ? AND status IN ('paid', 'uploaded')",
    )
      .bind(email)
      .all<BackupRow>();
    for (const b of results) await removeBackup(env, s, b, "You deleted your Ferry account.");
    await env.DB.batch([
      env.DB.prepare("DELETE FROM backups WHERE email = ?").bind(email),
      env.DB.prepare("DELETE FROM restore_codes WHERE identity = ?").bind(email),
      env.DB.prepare("DELETE FROM login_codes WHERE email = ?").bind(email),
      env.DB.prepare("DELETE FROM sessions WHERE email = ?").bind(email),
    ]);
    return json({ deleted: true });
  }

  if (get && path === "/api/cloud/backups") {
    const { results } = await env.DB.prepare(
      "SELECT * FROM backups WHERE email = ? AND status != 'unpaid' ORDER BY created DESC",
    )
      .bind(email)
      .all<BackupRow>();
    return json({ backups: results.map(publicView) });
  }

  if (post && path === "/api/cloud/checkout") return checkout(req, env, s, email, dev);

  const m = /^\/api\/cloud\/backups\/([^/]+)(?:\/([a-z-]+))?$/.exec(path);
  if (m && isValidBackupId(m[1])) {
    const backup = await env.DB.prepare("SELECT * FROM backups WHERE id = ? AND email = ?")
      .bind(m[1], email)
      .first<BackupRow>();
    // Someone else's backup looks exactly like a missing one.
    if (!backup) throw new HttpError(404, "Backup not found.");
    const action = m[2];

    if (!action && get) return json(publicView(backup));

    if (post && action === "upload-key") {
      if (backup.status !== "paid") throw new HttpError(409, uploadRefusal(backup.status));
      return json(await createUploadKey(env, backup.id));
    }
    if (post && action === "uploaded") {
      if (backup.status !== "paid") throw new HttpError(409, uploadRefusal(backup.status));
      const status = await finalize(env, s, backup);
      if (status === "paid") throw new HttpError(409, "No uploaded files were found for this backup.");
      if (status === "rejected") throw new HttpError(413, "The upload was larger than allowed, so it was deleted.");
      return json(publicView({ ...backup, status, expires: expiresFromNow(s) }));
    }
    if (post && action === "download-key") {
      if (backup.status !== "uploaded") throw new HttpError(409, "This backup is not available for download.");
      return json(await createDownloadKey(env, backup.id));
    }
    if (post && action === "delete") {
      await removeBackup(env, s, backup, "You deleted it. Nothing of yours remains on Ferry's servers.");
      return json({ deleted: true });
    }
  }

  throw new HttpError(404, "Not found.");
}

async function checkout(req: Request, env: Env, s: Settings, email: string, dev: boolean): Promise<Response> {
  const b = await body(req);
  const tier = s.cloudFree ? "50gb" : b.tier;
  if (!isTier(tier)) throw new HttpError(400, "Unknown plan size.");
  // Refuse up front when the app says the backup won't fit — otherwise the
  // user waits through a long upload only for `finalize` to delete it.
  if (typeof b.size === "number" && b.size > TIERS[tier].maxBytes) {
    throw new HttpError(413, `This backup is larger than ${s.cloudFree ? "the free 50 GB cloud limit" : "the chosen plan"}.`);
  }

  const active = (sql: string, ...args: unknown[]) =>
    env.DB.prepare(sql).bind(...args).first<{ n: number }>().then(r => r?.n ?? 0);
  if (s.cloudFree && (await active("SELECT COUNT(*) AS n FROM backups WHERE email = ? AND status IN ('paid', 'uploaded')", email))) {
    throw new HttpError(409, "You already have a cloud backup. Restore or delete it before making a new one.");
  }
  if ((await active("SELECT COUNT(*) AS n FROM backups WHERE status IN ('paid', 'uploaded')")) >= s.maxActiveBackups) {
    throw new HttpError(503, "Ferry Cloud is full right now. Your USB backup is complete — try the cloud copy again later.");
  }
  const ipHash = await sha256(req.headers.get("cf-connecting-ip") ?? "unknown");
  const today = await active(
    "SELECT COUNT(*) AS n FROM backups WHERE ip_hash = ? AND created > ?",
    ipHash,
    Date.now() - 86_400_000,
  );
  if (today >= s.perIpDaily) {
    throw new HttpError(429, "Too many cloud backups from this network today. Try again tomorrow.");
  }

  const id = crypto.randomUUID();
  const autoPaid = s.cloudFree || (dev && !env.LEMONSQUEEZY_API_KEY);
  await env.DB.prepare("INSERT INTO backups (id, email, tier, status, created, ip_hash) VALUES (?, ?, ?, ?, ?, ?)")
    .bind(id, email, tier, autoPaid ? "paid" : "unpaid", Date.now(), ipHash)
    .run();
  if (autoPaid) return json({ backupId: id, checkoutUrl: null });

  const variantId = variantFor(env, tier);
  if (!env.LEMONSQUEEZY_API_KEY || !env.LEMONSQUEEZY_STORE_ID || !variantId) {
    throw new HttpError(503, "Payments are not set up yet.");
  }
  if (email.startsWith("anon:")) throw new HttpError(400, "Paid backups need email sign-in.");
  const checkoutUrl = await createCheckout(env, { email, backupId: id, variantId });
  return json({ backupId: id, checkoutUrl });
}

function publicView(b: BackupRow) {
  const tier = isTier(b.tier) ? TIERS[b.tier].label : b.tier;
  return { id: b.id, tier, status: b.status, created: b.created, expires: b.expires };
}

function uploadRefusal(status: BackupRow["status"]): string {
  if (status === "unpaid") return "This backup hasn't been paid for yet.";
  if (status === "uploaded") return "This backup has already been uploaded.";
  return "This backup can no longer be uploaded.";
}

const expiresFromNow = (s: Settings) => Date.now() + s.retentionDays * 86_400_000;
const day = (ms: number) => new Date(ms).toISOString().slice(0, 10);

/** Checks what actually landed in B2 against the plan, then either confirms
 *  the backup (starting its retention clock) or deletes an oversized one. */
async function finalize(env: Env, s: Settings, b: BackupRow): Promise<BackupRow["status"]> {
  const size = await backupSize(env, b.id);
  if (size === 0) return b.status;
  if (isTier(b.tier) && size > TIERS[b.tier].maxBytes) {
    await deleteBackup(env, b.id);
    await env.DB.prepare("UPDATE backups SET status = 'rejected' WHERE id = ?").bind(b.id).run();
    await sendEmail(
      s,
      b.email,
      "Your Ferry cloud backup was too large",
      `The backup you uploaded was larger than ${TIERS[b.tier].label}, so it was deleted.\n\n` +
        `Your backup on the USB drive is not affected.`,
    ).catch(e => console.error("email failed", e));
    return "rejected";
  }
  const expires = expiresFromNow(s);
  await env.DB.prepare("UPDATE backups SET status = 'uploaded', expires = ? WHERE id = ?").bind(expires, b.id).run();
  await sendEmail(
    s,
    b.email,
    "Your Ferry cloud backup is safe — keep this email",
    `Your encrypted backup is stored in Ferry's cloud.\n\n` +
      `Backup ID: ${b.id}\n` +
      `Kept until: ${day(expires)}\n\n` +
      `To restore it on your new computer: open Ferry → Restore → Restore from Ferry Cloud → ` +
      `sign in with this email address → pick this backup.\n\n` +
      `You will also need your backup password. Ferry never had it and cannot recover it.`,
  ).catch(e => console.error("email failed", e));
  return "uploaded";
}

async function removeBackup(env: Env, s: Settings, b: BackupRow, why: string): Promise<void> {
  if (b.status === "deleted") return;
  await deleteBackup(env, b.id);
  await env.DB.prepare("UPDATE backups SET status = 'deleted' WHERE id = ?").bind(b.id).run();
  if (b.status === "uploaded") {
    await sendEmail(s, b.email, "Your Ferry cloud backup was deleted", `Backup ${b.id} was deleted. ${why}`).catch(e =>
      console.error("email failed", e),
    );
  }
}

// ── Admin ────────────────────────────────────────────────────────────────────

async function admin(req: Request, env: Env, s: Settings, path: string): Promise<Response> {
  const given = /^Bearer (.+)$/.exec(req.headers.get("authorization") ?? "")?.[1] ?? "";
  // Compare digests, not the strings: equal-length inputs, no early exit on
  // the secret itself.
  const byToken = Boolean(env.ADMIN_TOKEN) && (await sha256(given)) === (await sha256(env.ADMIN_TOKEN!));
  const signedIn = byToken ? null : await sessionEmail(env, req);
  if (!byToken && !(signedIn && s.adminEmails.includes(signedIn))) {
    throw new HttpError(401, "Not an admin. Sign in with an admin email, or use the admin token.");
  }
  const post = req.method === "POST";

  if (req.method === "GET" && path === "/admin/overview") {
    const count = (status: string) =>
      env.DB.prepare("SELECT COUNT(*) AS n FROM backups WHERE status = ?").bind(status).first<{ n: number }>().then(r => r?.n ?? 0);
    const { results } = await env.DB.prepare(
      "SELECT * FROM backups WHERE status != 'unpaid' ORDER BY created DESC LIMIT 200",
    ).all<BackupRow>();
    return json({
      settings: { ...s, resendApiKey: undefined, resendApiKeySet: Boolean(s.resendApiKey) },
      stats: { uploading: await count("paid"), stored: await count("uploaded"), deleted: await count("deleted") },
      backups: results.map(b => ({ ...publicView(b), owner: b.email.startsWith("anon:") ? "restore code" : b.email })),
    });
  }
  if (post && path === "/admin/settings") {
    const saved = await saveSettings(env, await body(req));
    return json({ saved });
  }
  if (post && path === "/admin/email-test") {
    const to = normalizeEmail((await body(req)).to);
    if (!to) throw new HttpError(400, "Enter a valid email address.");
    if (!s.resendApiKey) throw new HttpError(400, "Save a Resend API key first.");
    await sendEmail(s, to, "Ferry test email", "Email from Ferry is working.", true).catch(e => {
      throw new HttpError(502, `Resend refused the test: ${e instanceof Error ? e.message : e}`);
    });
    return json({ sent: true });
  }
  const m = /^\/admin\/backups\/([^/]+)\/delete$/.exec(path);
  if (post && m && isValidBackupId(m[1])) {
    const b = await env.DB.prepare("SELECT * FROM backups WHERE id = ?").bind(m[1]).first<BackupRow>();
    if (!b) throw new HttpError(404, "Backup not found.");
    await removeBackup(env, s, b, "It was removed by Ferry support.");
    return json({ deleted: true });
  }
  throw new HttpError(404, "Not found.");
}

// ── Payments (off while Cloud Backup is free) ───────────────────────────────

async function lemonWebhook(req: Request, env: Env): Promise<Response> {
  const raw = await req.text();
  const secret = env.LEMONSQUEEZY_WEBHOOK_SECRET;
  if (!secret || !(await verifyWebhook(secret, raw, req.headers.get("x-signature") ?? ""))) {
    throw new HttpError(401, "Invalid signature.");
  }
  const event = JSON.parse(raw) as {
    meta?: { event_name?: string; custom_data?: { backup_id?: unknown } };
    data?: { id?: string; attributes?: { status?: string; store_id?: number; first_order_item?: { variant_id?: number } } };
  };
  // Anything we don't act on still gets a 200, or Lemon Squeezy retries it.
  const id = event.meta?.custom_data?.backup_id;
  const attrs = event.data?.attributes;
  if (event.meta?.event_name !== "order_created" || !isValidBackupId(id) || attrs?.status !== "paid") {
    return json({ ignored: true });
  }
  const backup = await env.DB.prepare("SELECT * FROM backups WHERE id = ?").bind(id).first<BackupRow>();
  // The order must be for this store and for the tier the backup was created
  // with — a hand-edited checkout link can't buy 1 TB at the 50 GB price.
  const matches =
    backup &&
    isTier(backup.tier) &&
    String(attrs.store_id) === env.LEMONSQUEEZY_STORE_ID &&
    String(attrs.first_order_item?.variant_id) === variantFor(env, backup.tier);
  if (!matches) {
    console.warn(`Webhook for backup ${id} did not match its store/tier; ignored.`);
    return json({ ignored: true });
  }
  await env.DB.prepare("UPDATE backups SET status = 'paid', order_id = ? WHERE id = ? AND status = 'unpaid'")
    .bind(String(event.data?.id ?? ""), id)
    .run();
  return json({ ok: true });
}

// ── Housekeeping ─────────────────────────────────────────────────────────────

/** Hourly. Small batches: a free-plan Worker invocation may make only 50
 *  outbound requests, and each B2 delete takes several. */
async function sweep(env: Env): Promise<void> {
  const s = await loadSettings(env);
  const now = Date.now();
  await env.DB.batch([
    env.DB.prepare("DELETE FROM backups WHERE status = 'unpaid' AND created < ?").bind(now - 2 * 86_400_000),
    env.DB.prepare("DELETE FROM login_codes WHERE expires < ?").bind(now),
    env.DB.prepare("DELETE FROM sessions WHERE expires < ?").bind(now),
    env.DB.prepare("DELETE FROM rate_limits WHERE window_start < ?").bind(now - 86_400_000),
  ]);

  const batch = async (sql: string, ...args: unknown[]) =>
    (await env.DB.prepare(sql + " LIMIT 5").bind(...args).all<BackupRow>()).results;

  // Paid, but the app never reported the upload finished (closed mid-way):
  // once the 6-hour key has expired, settle it from what B2 actually holds.
  for (const b of await batch("SELECT * FROM backups WHERE status = 'paid' AND created < ?", now - 86_400_000)) {
    await finalize(env, s, b).catch(e => console.error(`finalize ${b.id}`, e));
  }

  for (const b of await batch(
    "SELECT * FROM backups WHERE status = 'uploaded' AND reminded = 0 AND expires BETWEEN ? AND ?",
    now,
    now + 3 * 86_400_000,
  )) {
    await sendEmail(
      s,
      b.email,
      "Your Ferry cloud backup will be deleted soon",
      `Backup ${b.id} will be deleted on ${day(b.expires!)}.\n\n` +
        `If you haven't restored it yet, open Ferry → Restore → Restore from Ferry Cloud before then.`,
    ).catch(e => console.error(`reminder ${b.id}`, e));
    await env.DB.prepare("UPDATE backups SET reminded = 1 WHERE id = ?").bind(b.id).run();
  }

  for (const b of await batch("SELECT * FROM backups WHERE status = 'uploaded' AND expires < ?", now)) {
    await removeBackup(env, s, b, "Its storage period ended.").catch(e => console.error(`expire ${b.id}`, e));
  }
}
