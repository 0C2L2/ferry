import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useTauriProgress } from "../hooks/useTauriProgress";
import { ProgressBar, StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { BackupLocation, ProgressPayload, RestoreSummary } from "../types";

interface Props {
  loc: BackupLocation;
  password: string;
  onDone: (summary: RestoreSummary, stagingDir: string) => void;
}

const STAGES = ["Decrypt", "Verify & copy files", "Reimport Wi-Fi"];

export function RestoreProgress({ loc, password, onDone }: Props) {
  const [states, setStates] = useState<StageState[]>(STAGES.map(() => "pending"));
  const [prog, setProg] = useState({ current: 0, total: 0 });
  const [item, setItem] = useState("Starting…");
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useTauriProgress("restore:progress", (p: ProgressPayload) => {
    setProg({ current: p.current, total: p.total });
    setItem(p.current_item);
  });

  function setStage(i: number, s: StageState) {
    setStates(prev => prev.map((v, idx) => (idx === i ? s : v)));
  }

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    (async () => {
      try {
        setStage(0, "active");
        const stagingDir = await api.decryptBackup(loc.drive_letter, password);
        setStage(0, "done");

        setStage(1, "active");
        const summary = await api.restoreFiles(stagingDir);
        setStage(1, "done");

        setStage(2, "active");
        const [wifiOk, wifiFail] = await api.importWifi(stagingDir);
        setStage(2, "done");
        onDone({ ...summary, wifi_restored: wifiOk, wifi_failed: wifiFail }, stagingDir);
      } catch (err) {
        setStates(prev => prev.map(v => (v === "active" ? "error" : v)));
        setError(String(err));
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Restoring your files…</h2>
      {error && (
        <ErrorBox
          message="Restore failed. Check the password and try again."
          detail={error}
        />
      )}
      <ProgressBar current={prog.current} total={prog.total} label={item} />
      <StageList stages={STAGES.map((name, i) => ({ name, state: states[i] }))} />
      <p className="text-xs text-gray-500 mt-4">
        Files land in a <span className="font-medium">Restored</span> folder on your desktop —
        never mixed into system folders.
      </p>
    </div>
  );
}
