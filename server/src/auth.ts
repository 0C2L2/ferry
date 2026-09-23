// Two ways to sign in, both ending in the same 30-day session:
//   • restore code — always available; needs no email. A random code shown
//     once in the app stands in for an address (identity 'anon:<uuid>').
//   • email code — a 6-digit code emailed via Resend; only while an admin has
//     switched email on (it needs Ferry's own domain to reach anyone).
// Nothing reusable is ever stored: codes and session tokens are kept only as
// SHA-256 hashes, so a leaked database cannot be used to sign in.

import { sendEmail, type MailConfig } from "./email";

export interface AuthEnv {
  DB: D1Database;
}

const CODE_TTL = 10 * 60 * 1000;
const SESSION_TTL = 30 * 24 * 60 * 60 * 1000;
const RESEND_COOLDOWN = 60 * 1000;
const MAX_ATTEMPTS = 5;

const hex = (bytes: ArrayBuffer | Uint8Array) =>
  [...new Uint8Array(bytes)].map(b => b.toString(16).padStart(2, "0")).join("");

export const sha256 = async (s: string) =>
  hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s)));

export function normalizeEmail(input: unknown): string | null {
  if (typeof input !== "string") return null;
  const email = input.trim().toLowerCase();
  return email.length <= 254 && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email) ? email : null;
}

async function createSession(env: AuthEnv, identity: string): Promise<string> {
  const token = hex(crypto.getRandomValues(new Uint8Array(32)));
  await env.DB.prepare("INSERT INTO sessions (token_hash, email, expires) VALUES (?, ?, ?)")
    .bind(await sha256(token), identity, Date.now() + SESSION_TTL)
    .run();
  return token;
}

// ── Restore code ─────────────────────────────────────────────────────────────

// No 0/O, 1/I/L: the code gets written on paper and typed back in.
const CODE_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";

/** `FERRY-XXXX-XXXX-XXXX-XXXX` → the 16 significant characters, or null. */
function canonicalRestoreCode(input: unknown): string | null {
  if (typeof input !== "string") return null;
  const chars = input.toUpperCase().replace(/[^0-9A-Z]/g, "").replace(/^FERRY/, "");
  return chars.length === 16 && [...chars].every(c => CODE_ALPHABET.includes(c)) ? chars : null;
}

/** A new identity with its restore code (~79 bits: not guessable). */
export async function startAnonymous(env: AuthEnv): Promise<{ token: string; restoreCode: string }> {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  const chars = [...bytes].map(b => CODE_ALPHABET[b % CODE_ALPHABET.length]).join("");
  const identity = `anon:${crypto.randomUUID()}`;
  await env.DB.prepare("INSERT INTO restore_codes (code_hash, identity, created) VALUES (?, ?, ?)")
    .bind(await sha256(chars), identity, Date.now())
    .run();
  const restoreCode = `FERRY-${chars.match(/.{4}/g)!.join("-")}`;
  return { token: await createSession(env, identity), restoreCode };
}

/** Who owns this restore code: 'anon:<uuid>', or an email once it has been
 *  linked to an account. Null for an unknown or malformed code. */
export async function restoreCodeOwner(env: AuthEnv, input: unknown): Promise<string | null> {
  const chars = canonicalRestoreCode(input);
  if (!chars) return null;
  const row = await env.DB.prepare("SELECT identity FROM restore_codes WHERE code_hash = ?")
    .bind(await sha256(chars))
    .first<{ identity: string }>();
  return row?.identity ?? null;
}

/** A session for whoever owns this restore code, or null. */
export async function signInWithRestoreCode(env: AuthEnv, input: unknown): Promise<string | null> {
  const owner = await restoreCodeOwner(env, input);
  return owner ? createSession(env, owner) : null;
}

// ── Email code ───────────────────────────────────────────────────────────────

/** Emails a fresh code. Returns the code itself only so dev mode can hand
 *  it back to local tests — callers must never expose it otherwise. */
export async function startLogin(
  env: AuthEnv,
  mail: MailConfig,
  email: string,
): Promise<{ ok: true; code: string } | { ok: false }> {
  const now = Date.now();
  const last = await env.DB.prepare("SELECT sent_at FROM login_codes WHERE email = ?")
    .bind(email)
    .first<{ sent_at: number }>();
  if (last && now - last.sent_at < RESEND_COOLDOWN) return { ok: false };

  // Per-address cooldown here; the per-network limit lives in index.ts.
  const code = String(crypto.getRandomValues(new Uint32Array(1))[0] % 1_000_000).padStart(6, "0");
  await env.DB.prepare(
    `INSERT INTO login_codes (email, code_hash, expires, attempts, sent_at) VALUES (?1, ?2, ?3, 0, ?4)
     ON CONFLICT(email) DO UPDATE SET code_hash = ?2, expires = ?3, attempts = 0, sent_at = ?4`,
  )
    .bind(email, await sha256(`${email}:${code}`), now + CODE_TTL, now)
    .run();
  try {
    await sendEmail(
      mail,
      email,
      `${code} is your Ferry sign-in code`,
      `Your Ferry sign-in code is ${code}\n\nIt expires in 10 minutes. If you didn't ask for it, you can ignore this email.`,
    );
  } catch (e) {
    // An unsent code must not hold the resend cooldown against the user.
    await env.DB.prepare("DELETE FROM login_codes WHERE email = ?").bind(email).run();
    throw e;
  }
  return { ok: true, code };
}

/** Returns a new session token, or null for a wrong/expired/over-tried code. */
export async function verifyLogin(env: AuthEnv, email: string, code: unknown): Promise<string | null> {
  const row = await env.DB.prepare("SELECT code_hash, expires, attempts FROM login_codes WHERE email = ?")
    .bind(email)
    .first<{ code_hash: string; expires: number; attempts: number }>();
  if (!row || row.expires < Date.now() || row.attempts >= MAX_ATTEMPTS) return null;

  if (typeof code !== "string" || (await sha256(`${email}:${code.trim()}`)) !== row.code_hash) {
    await env.DB.prepare("UPDATE login_codes SET attempts = attempts + 1 WHERE email = ?").bind(email).run();
    return null;
  }
  await env.DB.prepare("DELETE FROM login_codes WHERE email = ?").bind(email).run();
  return createSession(env, email);
}

// ── Sessions ─────────────────────────────────────────────────────────────────

const bearer = (req: Request) => /^Bearer ([0-9a-f]{64})$/.exec(req.headers.get("authorization") ?? "")?.[1];

/** The signed-in identity (an email, or 'anon:<uuid>') for the request, or null. */
export async function sessionEmail(env: AuthEnv, req: Request): Promise<string | null> {
  const token = bearer(req);
  if (!token) return null;
  const row = await env.DB.prepare("SELECT email, expires FROM sessions WHERE token_hash = ?")
    .bind(await sha256(token))
    .first<{ email: string; expires: number }>();
  return row && row.expires > Date.now() ? row.email : null;
}

export async function signOut(env: AuthEnv, req: Request): Promise<void> {
  const token = bearer(req);
  if (token) await env.DB.prepare("DELETE FROM sessions WHERE token_hash = ?").bind(await sha256(token)).run();
}
