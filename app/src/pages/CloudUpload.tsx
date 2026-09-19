import { useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api";
import { ProgressBar } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { DriveInfo } from "../types";
import { formatBytes } from "../types";

interface Props {
  drive: DriveInfo;
  onDone: () => void;
  onSkip: () => void;
}

interface CloudProgress {
  uploaded: number;
  total: number;
  part: number;
  parts: number;
}

/** "Extra Careful" — upload the already-encrypted Backup.enc to Backblaze B2.
 *  B2 never sees plaintext — our AES-256-GCM layer is still in place.
 *  The user supplies their own B2 Application Key (read-only bucket access).
 */
export function CloudUpload({ drive, onDone, onSkip }: Props) {
  const [keyId, setKeyId] = useState("");
  const [appKey, setAppKey] = useState("");
  const [bucket, setBucket] = useState("ferry-backup");
  const [busy, setBusy] = useState(false);
  const [prog, setProg] = useState<CloudProgress | null>(null);
  const [fileId, setFileId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function startUpload() {
    if (!keyId.trim() || !appKey.trim() || !bucket.trim()) return;
    setBusy(true);
    setError(null);
    setProg(null);

    // Listen to progress events while upload runs.
    const unlisten = await listen<CloudProgress>("cloud:progress", e => {
      setProg(e.payload);
    });

    try {
      const id = await api.uploadBackupB2(
        drive.drive_letter,
        keyId.trim(),
        appKey.trim(),
        bucket.trim(),
      );
      setFileId(id);
    } catch (err) {
      setError(String(err));
    } finally {
      unlisten();
      setBusy(false);
    }
  }

  if (fileId) {
    return (
      <div className="max-w-xl mx-auto text-center py-8">
        <div className="text-4xl mb-3">☁️</div>
        <h2 className="text-xl font-semibold mb-2">Cloud backup complete</h2>
        <p className="text-sm text-gray-500 mb-1">
          Backup.enc uploaded to B2 bucket <span className="font-medium">{bucket}</span>.
        </p>
        <p className="text-xs text-gray-400 mb-6">File ID: {fileId}</p>
        <button
          onClick={onDone}
          className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium hover:bg-brand-700"
        >
          Continue →
        </button>
      </div>
    );
  }

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Extra Careful — Cloud backup</h2>
      <p className="text-sm text-gray-500 mb-4">
        Upload your encrypted <code>Backup.enc</code> to Backblaze B2 for a second copy.
        B2 never sees your plaintext — the AES-256 layer stays in place.
      </p>
      <div className="bg-amber-50 border border-amber-200 rounded-lg p-3 mb-5 text-sm text-amber-900">
        You need a{" "}
        <a
          href="https://www.backblaze.com/b2/cloud-storage.html"
          target="_blank"
          rel="noopener noreferrer"
          className="underline"
        >
          Backblaze B2
        </a>{" "}
        account. Create an Application Key with <strong>read + write</strong> access to your
        bucket. First 10 GB/month and all egress is free.
      </div>
      {error && <ErrorBox message="Upload failed." detail={error} />}

      <label className="block text-sm font-medium mb-1">B2 Key ID</label>
      <input
        value={keyId}
        onChange={e => setKeyId(e.target.value)}
        placeholder="00abc123…"
        disabled={busy}
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3 text-sm font-mono"
      />

      <label className="block text-sm font-medium mb-1">B2 Application Key</label>
      <input
        type="password"
        value={appKey}
        onChange={e => setAppKey(e.target.value)}
        placeholder="K00…"
        disabled={busy}
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3 text-sm font-mono"
      />

      <label className="block text-sm font-medium mb-1">Bucket name</label>
      <input
        value={bucket}
        onChange={e => setBucket(e.target.value)}
        disabled={busy}
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-5 text-sm"
      />

      {prog && (
        <div className="mb-4">
          <ProgressBar
            current={prog.uploaded}
            total={prog.total}
            label={`Part ${prog.part}/${prog.parts} · ${formatBytes(prog.uploaded)} / ${formatBytes(prog.total)}`}
          />
        </div>
      )}

      <div className="flex gap-3">
        <button
          onClick={startUpload}
          disabled={busy || !keyId.trim() || !appKey.trim() || !bucket.trim()}
          className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
        >
          {busy ? "Uploading…" : "Upload to B2"}
        </button>
        <button
          onClick={onSkip}
          disabled={busy}
          className="px-4 py-2.5 text-sm text-gray-500 hover:text-gray-900"
        >
          Skip
        </button>
      </div>
    </div>
  );
}

