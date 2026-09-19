import { useEffect, useState } from "react";
import { api } from "../api";
import { ErrorBox } from "../components/ErrorBox";
import type { BackupLocation } from "../types";

interface Props {
  onFound: (loc: BackupLocation, password: string, cloudBackupId?: string) => void;
}

export function RestoreDetect({ onFound }: Props) {
  const [loc, setLoc] = useState<BackupLocation | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState("");

  const [fromCloud, setFromCloud] = useState(false);
  const [backupId, setBackupId] = useState("");
  const [cloudPassword, setCloudPassword] = useState("");
  const [cloudBusy, setCloudBusy] = useState(false);
  const [cloudError, setCloudError] = useState<string | null>(null);

  useEffect(() => {
    api
      .findBackup()
      .then(setLoc)
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  async function restoreFromCloud() {
    if (!backupId.trim() || !cloudPassword) return;
    setCloudBusy(true);
    setCloudError(null);
    try {
      const stagingDir = await api.downloadCloudBackup(backupId.trim());
      onFound(
        { drive_letter: stagingDir, enc_path: "", salt_path: "" },
        cloudPassword,
        backupId.trim(),
      );
    } catch (err) {
      setCloudError(String(err));
      setCloudBusy(false);
    }
  }

  if (fromCloud) {
    return (
      <div className="max-w-xl mx-auto">
        <h2 className="text-xl font-semibold mb-1">Restore from Ferry Cloud</h2>
        <p className="text-sm text-gray-500 mb-4">
          Enter the backup ID you were shown when you uploaded, and your encryption password.
        </p>
        {cloudError && <ErrorBox message="Could not restore from the cloud." detail={cloudError} />}
        <label className="block text-sm font-medium mb-1">Backup ID</label>
        <input
          value={backupId}
          onChange={e => setBackupId(e.target.value)}
          placeholder="e.g. 3fa85f64-5717-4562-b3fc-2c963f66afa6"
          disabled={cloudBusy}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3 text-sm font-mono"
        />
        <label className="block text-sm font-medium mb-1">Backup password</label>
        <input
          type="password"
          value={cloudPassword}
          onChange={e => setCloudPassword(e.target.value)}
          onKeyDown={e => {
            if (e.key === "Enter") void restoreFromCloud();
          }}
          disabled={cloudBusy}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-4"
        />
        <div className="flex gap-3">
          <button
            onClick={() => void restoreFromCloud()}
            disabled={cloudBusy || !backupId.trim() || !cloudPassword}
            className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
          >
            {cloudBusy ? "Downloading…" : "Download & restore →"}
          </button>
          <button
            onClick={() => setFromCloud(false)}
            disabled={cloudBusy}
            className="px-4 py-2.5 text-sm text-gray-500 hover:text-gray-900"
          >
            ← Back to USB restore
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Restore from USB</h2>
      <p className="text-sm text-gray-500 mb-4">
        Plug in the Ferry USB drive. Ferry looks for the encrypted backup automatically.
      </p>
      {error && <ErrorBox message="Backup detection failed." detail={error} />}
      {loading ? (
        <p className="text-gray-500 animate-pulse">Scanning removable drives…</p>
      ) : !loc ? (
        <p className="text-gray-500 mb-4">
          No Ferry backup found on any connected USB drive. Make sure the drive with
          Backup.enc is plugged in.
        </p>
      ) : (
        <>
          <div className="bg-white border border-gray-200 rounded-xl p-4 mb-4 text-sm">
            Backup found on <span className="font-medium">{loc.drive_letter}</span>
          </div>
          <label className="block text-sm font-medium mb-1">Backup password</label>
          <input
            type="password"
            value={password}
            onChange={e => setPassword(e.target.value)}
            onKeyDown={e => {
              if (e.key === "Enter" && password) onFound(loc, password);
            }}
            className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-4"
          />
          <button
            onClick={() => password && onFound(loc, password)}
            disabled={!password}
            className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
          >
            Decrypt & restore →
          </button>
        </>
      )}
      <button
        onClick={() => setFromCloud(true)}
        className="mt-4 block text-sm text-brand-700 hover:text-brand-600"
      >
        Lost your USB? Restore from Ferry Cloud instead →
      </button>
    </div>
  );
}
