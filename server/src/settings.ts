// Settings an admin can change from the website's admin page without a
// redeploy. Stored in D1; anything never set falls back to the defaults here
// (or to the matching wrangler var/secret, for values that predate the page).

export interface SettingsEnv {
  DB: D1Database;
  RESEND_API_KEY?: string;
  EMAIL_FROM?: string;
  CLOUD_FREE?: string;
  RETENTION_DAYS?: string;
}

export interface Settings {
  /** Email sign-in + backup emails. Off until Ferry has its own domain. */
  emailEnabled: boolean;
  resendApiKey: string;
  emailFrom: string;
  /** Where replies to Ferry's emails go (e.g. support@…); empty = no Reply-To. */
  emailReplyTo: string;
  /** Emails allowed into /admin after signing in (besides the ADMIN_TOKEN). */
  adminEmails: string[];
  cloudFree: boolean;
  /** Capped at 30 — the promise is "kept 30 days at most". */
  retentionDays: number;
  /** Across all users: when full, new cloud backups wait. */
  maxActiveBackups: number;
  /** New backups per network per day — the abuse brake while there is no email check. */
  perIpDaily: number;
}

const clamp = (n: number, lo: number, hi: number, fallback: number) =>
  Number.isFinite(n) ? Math.min(hi, Math.max(lo, Math.round(n))) : fallback;

export async function loadSettings(env: SettingsEnv): Promise<Settings> {
  const { results } = await env.DB.prepare("SELECT key, value FROM settings").all<{ key: string; value: string }>();
  const s: Record<string, string | undefined> = Object.fromEntries(results.map(r => [r.key, r.value]));
  return {
    emailEnabled: s.email_enabled === "1",
    resendApiKey: s.resend_api_key ?? env.RESEND_API_KEY ?? "",
    emailFrom: s.email_from ?? env.EMAIL_FROM ?? "Ferry <onboarding@resend.dev>",
    emailReplyTo: s.email_reply_to ?? "",
    adminEmails: (s.admin_emails ?? "").split(",").map(e => e.trim()).filter(Boolean),
    cloudFree: (s.cloud_free ?? env.CLOUD_FREE ?? "1") === "1",
    retentionDays: clamp(Number(s.retention_days ?? env.RETENTION_DAYS ?? 30), 1, 30, 30),
    maxActiveBackups: clamp(Number(s.max_active_backups ?? 50), 0, 100_000, 50),
    perIpDaily: clamp(Number(s.per_ip_daily ?? 3), 0, 1000, 3),
  };
}

/** Applies the fields present in `patch`; validates each. Returns what was saved. */
export async function saveSettings(env: SettingsEnv, patch: Record<string, unknown>): Promise<string[]> {
  const rows: [string, string][] = [];
  const bool = (v: unknown) => (v === true || v === "1" ? "1" : "0");
  if ("emailEnabled" in patch) rows.push(["email_enabled", bool(patch.emailEnabled)]);
  if ("cloudFree" in patch) rows.push(["cloud_free", bool(patch.cloudFree)]);
  if (typeof patch.resendApiKey === "string") rows.push(["resend_api_key", patch.resendApiKey.trim()]);
  if (typeof patch.emailFrom === "string" && patch.emailFrom.trim()) rows.push(["email_from", patch.emailFrom.trim()]);
  if (typeof patch.emailReplyTo === "string") rows.push(["email_reply_to", patch.emailReplyTo.trim()]);
  if (typeof patch.adminEmails === "string" || Array.isArray(patch.adminEmails)) {
    const list = (Array.isArray(patch.adminEmails) ? patch.adminEmails : patch.adminEmails.split(","))
      .map(e => String(e).trim().toLowerCase())
      .filter(e => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e));
    rows.push(["admin_emails", [...new Set(list)].join(",")]);
  }
  if ("retentionDays" in patch) rows.push(["retention_days", String(clamp(Number(patch.retentionDays), 1, 30, 30))]);
  if ("maxActiveBackups" in patch) rows.push(["max_active_backups", String(clamp(Number(patch.maxActiveBackups), 0, 100_000, 50))]);
  if ("perIpDaily" in patch) rows.push(["per_ip_daily", String(clamp(Number(patch.perIpDaily), 0, 1000, 3))]);
  if (rows.length) {
    await env.DB.batch(
      rows.map(([k, v]) =>
        env.DB.prepare("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value").bind(k, v),
      ),
    );
  }
  return rows.map(([k]) => k);
}
