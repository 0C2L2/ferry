import { useEffect, useState } from "react";
import { api } from "../api";
import { ErrorBox } from "../components/ErrorBox";
import type { BackupLocation } from "../types";

interface Props {
  onFound: (loc: BackupLocation, password: string) => void;
}

export function RestoreDetect({ onFound }: Props) {
  const [loc, setLoc] = useState<BackupLocation | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState("");

  useEffect(() => {
    api
      .findBackup()
      .then(setLoc)
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

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
        <p className="text-gray-500">
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
    </div>
  );
}
