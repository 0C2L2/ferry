import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useTauriProgress } from "../hooks/useTauriProgress";
import { ProgressBar, StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { DriveInfo, Manifest, ProgressPayload, ScanResult } from "../types";

interface Props {
  drive: DriveInfo;
  scan: ScanResult;
  password: string;
  onDone: (manifest: Manifest) => void;
}

const STAGES = ["Copy files", "Browser data", "Wi-Fi profiles", "App & driver list", "Verify", "Encrypt"];

export function BackupProgress({ drive, scan, password, onDone }: Props) {
  const [states, setStates] = useState<StageState[]>(STAGES.map(() => "pending"));
  const [detail, setDetail] = useState("");
  const [prog, setProg] = useState({ current: 0, total: 0 });
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useTauriProgress("backup:progress", (p: ProgressPayload) => {
    setProg({ current: p.current, total: p.total });
    setDetail(p.current_item);
  });

  function setStage(i: number, s: StageState) {
    setStates(prev => prev.map((v, idx) => (idx === i ? s : v)));
  }

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    (async () => {
      try {
        const root = drive.drive_letter;
        setStage(0, "active");
        await api.copyFiles(scan.files, root);
        setStage(0, "done");

        setStage(1, "active");
        const [chromium, firefox] = await Promise.all([
          api.backupChromium(root),
          api.backupFirefox(root),
        ]);
        const browserSkipped = [...chromium.skipped, ...firefox.skipped];
        setStage(1, browserSkipped.length ? "done" : "done");

        setStage(2, "active");
        const wifiCount = await api.exportWifi(root);
        setStage(2, "done");

        setStage(3, "active");
        const apps = await api.scanApps();
        const tiered = await api.resolveTiers(apps);
        const drivers = await api.scanDrivers();
        await api.saveInventory(root, JSON.stringify(tiered), JSON.stringify(drivers));
        setStage(3, "done");

        setStage(4, "active");
        const manifest = await api.verifyBackup(scan.files, root, scan.skipped_reasons);
        setStage(4, "done");

        setStage(5, "active");
        await api.encryptBackup(root, password);
        setStage(5, "done");
        setDetail(
          `Backed up ${manifest.files.length} files, ${wifiCount} Wi-Fi networks, ` +
            `${chromium.backed_up.length + firefox.backed_up.length} browser files` +
            (browserSkipped.length ? ` (${browserSkipped.length} browser files skipped)` : ""),
        );
        onDone(manifest);
      } catch (err) {
        setStates(prev => prev.map(v => (v === "active" ? "error" : v)));
        setError(String(err));
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Backing up…</h2>
      {error && <ErrorBox message="Backup failed. Nothing was erased." detail={error} />}
      <ProgressBar current={prog.current} total={prog.total} label={detail || "Working…"} />
      <StageList stages={STAGES.map((name, i) => ({ name, state: states[i] }))} />
      <p className="text-xs text-gray-500 mt-4">
        Do not remove the USB drive until this finishes.
      </p>
    </div>
  );
}
