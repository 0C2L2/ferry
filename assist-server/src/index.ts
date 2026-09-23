import "dotenv/config";
import express from "express";
import cors from "cors";
import { createUploadKey, createDownloadKey, deleteBackup, isValidBackupId } from "./services/b2admin.js";

const app = express();
app.use(cors());
app.use(express.json({ limit: "1mb" }));

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
    res.json({ deleted: true });
  } catch (err) {
    res.status(502).json({ error: err instanceof Error ? err.message : String(err) });
  }
});

const port = Number(process.env.PORT ?? 8787);
app.listen(port, () => {
  console.log(`Ferry server listening on http://localhost:${port}`);
  printReadiness();
});

/** Readiness probe: which features have credentials, and exactly what is
 *  still missing for the rest. The server always boots — unconfigured
 *  features fail with a clear error only when called. */
app.get("/ready", (_req, res) => {
  res.json({ features: featureStatus() });
});

interface FeatureStatus {
  feature: string;
  ready: boolean;
  missing: string[];
}

function featureStatus(): FeatureStatus[] {
  const need = (...keys: string[]): string[] =>
    keys.filter(k => !process.env[k]);
  return [
    {
      feature: "Cloud Backup (B2 upload/download/delete)",
      missing: need("B2_MASTER_KEY_ID", "B2_MASTER_APPLICATION_KEY", "B2_BUCKET_NAME"),
      ready: false,
    },
  ].map(f => ({ ...f, ready: f.missing.length === 0 }));
}

function printReadiness() {
  for (const f of featureStatus()) {
    if (f.ready) console.log(`  [ready] ${f.feature}`);
    else console.log(`  [missing] ${f.feature} — needs: ${f.missing.join(", ")}`);
  }
}
