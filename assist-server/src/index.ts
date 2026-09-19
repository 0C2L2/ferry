import "dotenv/config";
import express from "express";
import cors from "cors";
import { suggestForApps } from "./services/nosana.js";
import { verifyCommand } from "./services/daytona.js";
import { createShareLink } from "./services/dnsimple.js";
import { createUploadKey, createDownloadKey, deleteBackup, isValidBackupId } from "./services/b2admin.js";

const app = express();
app.use(cors());
app.use(express.json({ limit: "1mb" }));

// In-memory only — this is a stateless hackathon sidecar, not a database.
// Status entries are lost on restart; that's fine for a demo share link.
const backupStatuses = new Map<string, { createdAt: string }>();

app.post("/api/assist/suggest", async (req, res) => {
  try {
    const { apps, targetFamily } = req.body ?? {};
    if (!Array.isArray(apps) || apps.length === 0) {
      res.status(400).json({ error: "apps must be a non-empty array" });
      return;
    }
    const suggestions = await suggestForApps(apps, typeof targetFamily === "string" ? targetFamily : "unknown");
    res.json({ suggestions });
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

app.post("/api/assist/verify", async (req, res) => {
  try {
    const { command } = req.body ?? {};
    if (typeof command !== "string" || !command.trim()) {
      res.status(400).json({ error: "command must be a non-empty string" });
      return;
    }
    const result = await verifyCommand(command);
    res.json(result);
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

app.post("/api/cloud/share-link", async (req, res) => {
  try {
    const { backupId } = req.body ?? {};
    if (!isValidBackupId(backupId)) {
      res.status(400).json({ error: "backupId must be a valid backup ID" });
      return;
    }
    backupStatuses.set(backupId, { createdAt: new Date().toISOString() });
    const link = await createShareLink(backupId);
    res.json(link);
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

// ── Ferry Cloud Backup (managed) ────────────────────────────────────────────
// The desktop app never sees Ferry's B2 master key. Each of these mints a
// disposable, narrowly-scoped B2 Application Key instead — see b2admin.ts.

app.post("/api/cloud/upload-key", async (_req, res) => {
  try {
    const key = await createUploadKey();
    res.json(key);
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

app.post("/api/cloud/download-key", async (req, res) => {
  try {
    const { backupId } = req.body ?? {};
    if (!isValidBackupId(backupId)) {
      res.status(400).json({ error: "backupId must be a valid backup ID" });
      return;
    }
    const key = await createDownloadKey(backupId);
    res.json(key);
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

app.post("/api/cloud/delete", async (req, res) => {
  try {
    const { backupId } = req.body ?? {};
    if (!isValidBackupId(backupId)) {
      res.status(400).json({ error: "backupId must be a valid backup ID" });
      return;
    }
    await deleteBackup(backupId);
    backupStatuses.delete(backupId);
    res.json({ deleted: true });
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

app.get("/status/:backupId", (req, res) => {
  const status = backupStatuses.get(req.params.backupId);
  if (!status) {
    res.status(404).send("Unknown or expired backup link.");
    return;
  }
  res
    .type("html")
    .send(
      `<!doctype html><html><head><meta charset="utf-8"><title>Ferry Cloud Backup</title>` +
        `<style>body{font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1rem;color:#1f2937}` +
        `.badge{display:inline-block;background:#dcfce7;color:#166534;padding:.25rem .75rem;border-radius:999px;font-size:.875rem}` +
        `</style></head><body>` +
        `<h1>☁️ Cloud backup complete</h1>` +
        `<p class="badge">Uploaded successfully</p>` +
        `<p>Backup ID: <code>${escapeHtml(req.params.backupId)}</code></p>` +
        `<p>Completed: ${escapeHtml(status.createdAt)}</p>` +
        `</body></html>`,
    );
});

function escapeHtml(s: string): string {
  const map: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
  return s.replace(/[&<>"']/g, c => map[c]);
}

const port = Number(process.env.PORT ?? 8787);
app.listen(port, () => {
  console.log(`Ferry Assist server listening on http://localhost:${port}`);
});
