import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { CloudSignIn } from "../components/CloudSignIn";
import { ProgressBar } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import { formatBytes, type CloudStatus } from "../types";

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
 *  Ferry's own cloud storage (server: ../../../server). Two ways in:
 *    • restore code (default) — the server issues a code; it is the only key
 *      to the backup, so the user must save it before moving on;
 *    • email (when an admin has switched it on) — sign in with an emailed code.
 *  Free for now: up to 50 GB, one backup at a time, kept 30 days. */
export function CloudUpload({ dataRoot, onDone, onSkip }: Props) {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const [email, setEmail] = useState<string | null>(null);
  const [restoreCode, setRestoreCode] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [prog, setProg] = useState<CloudProgress | null>(null);
  const [backupId, setBackupId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    api
      .cloudStatus()
      .then(s => {
        setStatus(s);
        setEmail(s.email);
      })
      .catch(() => {});
  }, []);

  const emailMode = status?.emailSignIn ?? false;

  async function startUpload() {
    setBusy(true);
    setError(null);
    setProg(null);
    const unlisten = await listen<CloudProgress>("cloud:progress", e => setProg(e.payload));
    try {
      if (!emailMode && !restoreCode) setRestoreCode(await api.cloudStartAnonymous());
      setBackupId(await api.uploadBackupB2(dataRoot));
    } catch (err) {
      setError(String(err));
    } finally {
      unlisten();
      setBusy(false);
    }
  }

  async function saveCode() {
    if (!restoreCode) return;
    const path = await save({
      defaultPath: "Ferry restore code.txt",
      filters: [{ name: "Text file", extensions: ["txt"] }],
    });
    if (!path) return;
    try {
      await api.saveRestoreCode(path, restoreCode);
      setNotice(`Saved to ${path}`);
    } catch (err) {
      setNotice(String(err));
    }
  }

  if (backupId && restoreCode) {
    return (
      <div className="max-w-xl mx-auto py-6">
        <div className="text-center">
          <div className="text-4xl mb-3">☁️</div>
          <h2 className="text-xl font-semibold mb-2">Cloud backup complete — save your restore code</h2>
          <p className="text-sm text-gray-500 mb-4">
            This code is the <strong>only</strong> way to reach your cloud copy from another computer.
            Ferry can't recover it for you.
          </p>
        </div>
        <div className="bg-amber-50 border border-amber-200 rounded-xl p-5 mb-4">
          <p className="font-mono text-2xl text-center tracking-wider select-all break-all">{restoreCode}</p>
          <div className="flex justify-center gap-3 mt-4">
            <button
              onClick={() => void navigator.clipboard.writeText(restoreCode).then(() => setNotice("Copied."))}
              className="px-4 py-2 rounded-lg border border-amber-300 bg-white text-sm hover:bg-amber-100"
            >
              Copy
            </button>
            <button
              onClick={() => void saveCode()}
              className="px-4 py-2 rounded-lg border border-amber-300 bg-white text-sm hover:bg-amber-100"
            >
              Save as a file…
            </button>
          </div>
          {notice && <p className="text-xs text-amber-800 text-center mt-2 break-all">{notice}</p>}
          <p className="text-xs text-amber-800 mt-4">
            Write it down, or save it somewhere that is <strong>not</strong> the Ferry USB — a photo on
            your phone, or an email to yourself. On the new computer: Ferry → Restore → Restore from
            Ferry Cloud → this code → your backup password. Kept 30 days.
          </p>
        </div>
        <label className="flex items-center gap-2 text-sm mb-4">
          <input type="checkbox" checked={saved} onChange={e => setSaved(e.target.checked)} />
          I've saved my restore code somewhere safe.
        </label>
        <button
          onClick={onDone}
          disabled={!saved}
          className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
        >
          Continue →
        </button>
      </div>
    );
  }

  if (backupId) {
    return (
      <div className="max-w-xl mx-auto text-center py-8">
        <div className="text-4xl mb-3">☁️</div>
        <h2 className="text-xl font-semibold mb-2">Cloud backup complete</h2>
        <p className="text-sm text-gray-500 mb-4">
          Your encrypted backup is in Ferry's cloud for 30 days. We emailed the details to{" "}
          <strong>{email}</strong>.
        </p>
        <div className="bg-amber-50 border border-amber-200 rounded-lg p-4 mb-6 text-left">
          <p className="text-sm text-amber-900">
            To restore it: Ferry → Restore → <em>Restore from Ferry Cloud</em> → sign in with {email}.
            You'll also need your backup password — Ferry never had it and can't recover it.
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
        Keep a second copy of your encrypted backup in Ferry's cloud, in case the USB is lost or
        damaged. <strong>Free for now</strong> — up to 50 GB, kept for 30 days, deleted once you've
        restored it. Ferry never sees your files: the password you set encrypts them before upload.
      </p>

      {emailMode &&
        (!email ? (
          <CloudSignIn purpose="to back up to Ferry Cloud" onSignedIn={setEmail} />
        ) : (
          <p className="text-sm text-gray-600 mb-4">
            Signed in as <strong>{email}</strong>.
          </p>
        ))}
      {!emailMode && status && (
        <p className="text-sm text-gray-600 mb-4">
          No account needed: after the upload you get a <strong>restore code</strong> to keep safe.
        </p>
      )}

      {error &&
        (error.includes("Could not reach the Ferry Cloud service") ? (
          <ErrorBox
            message="Ferry Cloud can't be reached right now. Your USB backup is already complete and safe — you can continue without the cloud copy."
            detail={error}
          />
        ) : (
          <ErrorBox message="Upload failed. Your USB backup is not affected." detail={error} />
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
          onClick={() => void startUpload()}
          disabled={busy || !status || (emailMode && !email)}
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
