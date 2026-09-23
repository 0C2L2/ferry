import { useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api";
import { ProgressBar } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import { formatBytes } from "../types";

interface Props {
  dataRoot: string;
  onDone: () => void;
  onSkip: () => void;
}

interface CloudProgress {
  uploaded: number;
  total: number;
  part: number;
  parts: number;
}

/** "Extra Careful" — upload the already-encrypted Backup.enc + backup.salt to
 *  Ferry's own managed cloud storage. Cloud Backup is the one paid add-on
 *  (one-time per migration, priced by backup size — see Pricing and
 *  company/business-model.md); sign-in at checkout arrives with the accounts
 *  launch, and uploads run open until then. The only thing the user must keep
 *  is the backup ID this returns — same importance as their encryption
 *  password. Ferry mints a disposable, backup-scoped credential behind the
 *  scenes (see assist-server/src/services/b2admin.ts) so the user never
 *  touches a real B2 key.
 */
export function CloudUpload({ dataRoot, onDone, onSkip }: Props) {
  const [busy, setBusy] = useState(false);
  const [prog, setProg] = useState<CloudProgress | null>(null);
  const [backupId, setBackupId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function startUpload() {
    setBusy(true);
    setError(null);
    setProg(null);

    // Listen to progress events while upload runs.
    const unlisten = await listen<CloudProgress>("cloud:progress", e => {
      setProg(e.payload);
    });

    try {
      const id = await api.uploadBackupB2(dataRoot);
      setBackupId(id);
    } catch (err) {
      setError(String(err));
    } finally {
      unlisten();
      setBusy(false);
    }
  }

  if (backupId) {
    return (
      <div className="max-w-xl mx-auto text-center py-8">
        <div className="text-4xl mb-3">☁️</div>
        <h2 className="text-xl font-semibold mb-2">Cloud backup complete</h2>
        <p className="text-sm text-gray-500 mb-4">
          Your encrypted backup is safe in Ferry's cloud storage. Cloud Backup
          is a paid add-on — one-time per migration, priced by backup size
          (see Pricing).
        </p>
        <div className="bg-amber-50 border border-amber-200 rounded-lg p-4 mb-6 text-left">
          <p className="text-sm font-medium text-amber-900 mb-1">
            Write this down — you'll need it to restore from the cloud if you lose this USB:
          </p>
          <p className="font-mono text-sm bg-white border border-amber-200 rounded px-3 py-2 break-all select-all">
            {backupId}
          </p>
          <p className="text-xs text-amber-700 mt-2">
            You'll also need your encryption password. Without both, this cloud copy can't be
            recovered — Ferry doesn't keep a separate record of either.
          </p>
        </div>

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
        Upload your encrypted backup to Ferry's cloud storage for a second copy.
        One-time charge per migration (see Pricing for sizes) — checkout opens
        with the accounts launch, and uploads run open until then. Ferry never
        sees your plaintext; the AES-256 layer you already set stays in place
        the whole way, and the copy is deleted once you've restored it.
      </p>
      {error &&
        (error.includes("Could not reach the Ferry Cloud service") ? (
          <ErrorBox
            message="Ferry Cloud can't be reached right now. Your USB backup is already complete and safe — you can continue without the cloud copy."
            detail={error}
          />
        ) : (
          <ErrorBox message="Upload failed." detail={error} />
        ))}

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
          disabled={busy}
          className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
        >
          {busy ? "Uploading…" : "Back up to Ferry Cloud"}
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
