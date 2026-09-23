import { useEffect, useState } from "react";
import { api } from "../api";
import { CloudSignIn } from "../components/CloudSignIn";
import { ErrorBox } from "../components/ErrorBox";
import type { BackupLocation, CloudBackup } from "../types";

interface Props {
  onFound: (loc: BackupLocation, password: string, cloudBackupId?: string) => void;
}

export function RestoreDetect({ onFound }: Props) {
  const [loc, setLoc] = useState<BackupLocation | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [password, setPassword] = useState("");

  const [cloudReachable, setCloudReachable] = useState(false);
  const [emailMode, setEmailMode] = useState(false);
  const [fromCloud, setFromCloud] = useState(false);
  const [signedIn, setSignedIn] = useState(false);
  const [restoreCode, setRestoreCode] = useState("");
  const [backups, setBackups] = useState<CloudBackup[] | null>(null);
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
    api
      .cloudStatus()
      .then(s => {
        setCloudReachable(s.reachable);
        setEmailMode(s.emailSignIn);
        setSignedIn(s.signedIn);
      })
      .catch(() => {});
  }, []);

  // Once signed in, list the backups this account can restore.
  useEffect(() => {
    if (!fromCloud || !signedIn) return;
    api
      .cloudListBackups()
      .then(all => {
        const ready = all.filter(b => b.status === "uploaded");
        setBackups(ready);
        if (ready.length > 0) setBackupId(ready[0].id);
      })
      .catch(err => setCloudError(String(err)));
  }, [fromCloud, signedIn]);

  async function signInWithCode() {
    setCloudBusy(true);
    setCloudError(null);
    try {
      await api.cloudSignInCode(restoreCode.trim());
      setSignedIn(true);
    } catch (err) {
      setCloudError(String(err));
    } finally {
      setCloudBusy(false);
    }
  }

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
          {emailMode
            ? "Sign in with the email you used when backing up, pick the backup, and enter your backup password."
            : "Enter the restore code Ferry gave you after the cloud upload, then your backup password."}
        </p>
        {cloudError && <ErrorBox message="Could not restore from the cloud." detail={cloudError} />}
        {!signedIn && emailMode && (
          <CloudSignIn purpose="to find your cloud backup" onSignedIn={() => setSignedIn(true)} />
        )}
        {!signedIn && !emailMode && (
          <div className="flex gap-2 mb-4">
            <input
              value={restoreCode}
              onChange={e => setRestoreCode(e.target.value.toUpperCase())}
              onKeyDown={e => e.key === "Enter" && restoreCode.trim() && void signInWithCode()}
              placeholder="FERRY-XXXX-XXXX-XXXX-XXXX"
              disabled={cloudBusy}
              className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono"
            />
            <button
              onClick={() => void signInWithCode()}
              disabled={cloudBusy || !restoreCode.trim()}
              className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium disabled:opacity-40 hover:bg-brand-700"
            >
              {cloudBusy ? "Checking…" : "Find my backup"}
            </button>
          </div>
        )}
        {signedIn && backups === null && !cloudError && (
          <p className="text-sm text-gray-500 animate-pulse mb-3">Looking for your backups…</p>
        )}
        {signedIn && backups?.length === 0 && (
          <p className="text-sm text-gray-600 mb-3">
            No cloud backups found. They're kept for 30 days after upload.
          </p>
        )}
        {backups && backups.length > 0 && (
          <>
            <label className="block text-sm font-medium mb-1">Backup</label>
            <select
              value={backupId}
              onChange={e => setBackupId(e.target.value)}
              disabled={cloudBusy}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3 text-sm"
            >
              {backups.map(b => (
                <option key={b.id} value={b.id}>
                  Uploaded {new Date(b.created).toLocaleDateString()}
                  {b.expires ? ` · kept until ${new Date(b.expires).toLocaleDateString()}` : ""}
                </option>
              ))}
            </select>
          </>
        )}
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
      {cloudReachable && (
        <button
          onClick={() => setFromCloud(true)}
          className="mt-4 block text-sm text-brand-700 hover:text-brand-600"
        >
          Lost your USB? Restore from Ferry Cloud instead →
        </button>
      )}
    </div>
  );
}
