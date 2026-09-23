/** The slice of Settings that sending needs (see settings.ts). */
export interface MailConfig {
  emailEnabled: boolean;
  resendApiKey: string;
  emailFrom: string;
  emailReplyTo?: string;
}

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** The same message as HTML: Ferry header, paragraphs, and any 6-digit code
 *  or `FERRY-…`/UUID token set in a monospace box so it's easy to copy. */
function html(subject: string, text: string): string {
  const paragraphs = text
    .split(/\n{2,}/)
    .map(p => {
      const body = escape(p).replace(/\n/g, "<br>");
      return /^(\d{6}|Backup ID: .+)$/.test(p.trim())
        ? `<p style="font:600 22px/1.4 ui-monospace,Consolas,monospace;letter-spacing:2px;background:#eef2ff;color:#312e81;padding:14px 18px;border-radius:10px;margin:18px 0">${body}</p>`
        : `<p style="margin:0 0 14px">${body}</p>`;
    })
    .join("");
  return `<!doctype html><html><body style="margin:0;background:#f6f7fb;padding:32px 16px;font:16px/1.6 system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;color:#14161f">
<div style="max-width:520px;margin:0 auto;background:#fff;border:1px solid #e4e6ee;border-radius:14px;padding:28px">
<div style="font-weight:700;font-size:20px;margin-bottom:18px">Ferry</div>
<h1 style="font-size:18px;margin:0 0 14px">${escape(subject)}</h1>
${paragraphs}
</div>
<p style="max-width:520px;margin:14px auto 0;color:#5b6174;font-size:13px">Ferry · ferryapp.download — you get this because this address was used with Ferry.</p>
</body></html>`;
}

/** Sends through Resend (plain text + HTML). Skipped (only logged) while
 *  email is switched off in admin settings or no key is set, and always for
 *  restore-code identities, which have no inbox. `force` lets the admin page
 *  send a test before switching email on. */
export async function sendEmail(
  cfg: MailConfig,
  to: string,
  subject: string,
  text: string,
  force = false,
): Promise<void> {
  if (to.startsWith("anon:")) return;
  if ((!cfg.emailEnabled && !force) || !cfg.resendApiKey) {
    console.log(`[email → ${to}] ${subject}\n${text}`);
    return;
  }
  // Put a code on its own paragraph so the HTML version can box it.
  const body = text.replace(/(is )(\d{6})(\n)/, "$1\n\n$2\n$3");
  const res = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: { Authorization: `Bearer ${cfg.resendApiKey}`, "content-type": "application/json" },
    body: JSON.stringify({
      from: cfg.emailFrom,
      to: [to],
      subject,
      text,
      html: html(subject, body),
      ...(cfg.emailReplyTo ? { reply_to: cfg.emailReplyTo } : {}),
    }),
  });
  if (!res.ok) throw new Error(`Resend failed: ${res.status} ${await res.text()}`);
}
