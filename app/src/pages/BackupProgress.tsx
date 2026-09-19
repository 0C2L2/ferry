import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useTauriProgress } from "../hooks/useTauriProgress";
import { ProgressBar, StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { Manifest, ProgressPayload, ScanResult } from "../types";

interface Props {
  dataRoot: string;
  scan: ScanResult;
  password: string;
  onDone: (manifest: Manifest) => void;
}

const STAGES = ["Copy files", "Browser data", "Wi-Fi profiles", "App & driver list", "Verify", "Encrypt"];

export function BackupProgress({ dataRoot, scan, password, onDone }: Props) {
  const [states, setStates] = useState<StageState[]>(STAGES.map(() => "pending"));
  const [detail, setDetail] = useState("");
  const [prog, setProg] = useState({ current: 0, total: 0 });
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [finished, setFinished] = useState(false);
  const [succeeded, setSucceeded] = useState(false);
  const [elapsed, setElapsed] = useState(0);
  const [ago, setAgo] = useState(0);
  const started = useRef(false);
  const lastEvent = useRef<number>(Date.now());

  const STAGE_LABELS: Record<string, string> = {
    copy: "",
    verify: "",
    inventory: "Catalog",
    download: "Download",
    restore: "Restore",
  };

  useTauriProgress("backup:progress", (p: ProgressPayload) => {
    lastEvent.current = Date.now();
    setProg({ current: p.current, total: p.total });
    const prefix = STAGE_LABELS[p.stage] ?? p.stage;
    const label = prefix ? `${prefix} ${p.current}/${p.total}: ${p.current_item}` : p.current_item;
    setDetail(label);
  });

  // Ticking clock: proves the app is alive even during silent stages
  // (decrypt, winget lookups) and shows how stale the last update is.
  useEffect(() => {
    if (finished) return;
    const t0 = Date.now();
    const timer = setInterval(() => {
      setElapsed(Math.floor((Date.now() - t0) / 1000));
      setAgo(Math.floor((Date.now() - lastEvent.current) / 1000));
    }, 1000);
    return () => clearInterval(timer);
  }, [finished]);

  function fmt(s: number) {
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }

  function setStage(i: number, s: StageState) {
    setStates(prev => prev.map((v, idx) => (idx === i ? s : v)));
  }

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    (async () => {
      // Auxiliary steps (browsers, Wi-Fi, inventory) must never kill a backup:
      // machines without Wi-Fi, browsers, or winget still get their files.
      const aux = async <T,>(label: string, fn: () => Promise<T>): Promise<T | null> => {
        try {
          return await fn();
        } catch (err) {
          setWarnings(prev => [...prev, `${label}: ${String(err)}`]);
          return null;
        }
      };
      try {
        const root = dataRoot;
        setStage(0, "active");
        await api.copyFiles(scan.files, root);
        setStage(0, "done");

        setStage(1, "active");
        setDetail("Backing up browser profiles…");
        const browserSkipped: string[] = [];
        const chromium = await aux("Browser data", async () => {
          const r = await api.backupChromium(root);
          browserSkipped.push(...r.skipped);
          return `${r.backed_up.length} files`;
        });
        const firefox = await aux("Browser data", async () => {
          const r = await api.backupFirefox(root);
          browserSkipped.push(...r.skipped);
          return `${r.backed_up.length} files`;
        });
        if (chromium === null && firefox === null) {
          setStage(1, "error");
        } else {
          setStage(1, "done");
        }

        setStage(2, "active");
        setDetail("Exporting Wi-Fi profiles…");
        const wifiCount = await aux("Wi-Fi export", () => api.exportWifi(root));
        setStage(2, wifiCount === null ? "error" : "done");

        setStage(3, "active");
        setDetail("Scanning installed apps…");
        const inventoryOk = await aux("App & driver list", async () => {
          const apps = await api.scanApps();
          const tiered = await api.resolveTiers(apps);
          const drivers = await api.scanDrivers();
          await api.saveInventory(root, JSON.stringify(tiered), JSON.stringify(drivers));
          return `${tiered.length} apps, ${drivers.length} drivers`;
        });
        setStage(3, inventoryOk === null ? "error" : "done");

        // Index everything now on the USB (browser data, Wi-Fi incl. the
        // password sheet, inventory) so it joins the manifest — otherwise it
        // would be encrypted but never verified or restored.
        const usbFiles = await api.listUsbFiles(root);
        const allFiles = [...scan.files, ...usbFiles];

        setStage(4, "active");
        const manifest = await api.verifyBackup(allFiles, root, scan.skipped_reasons);
        setStage(4, "done");

        setStage(5, "active");
        await api.encryptBackup(root, password);
        setStage(5, "done");
        setSucceeded(true);
        setFinished(true);
        setDetail(
          `Backed up ${manifest.files.length} files` +
            (browserSkipped.length ? ` (${browserSkipped.length} browser files skipped)` : ""),
        );
        onDone(manifest);
      } catch (err) {
        setStates(prev => prev.map(v => (v === "active" ? "error" : v)));
        setFinished(true);
        setError(String(err));
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Backing up…</h2>
      {error && (
        <ErrorBox
          message="Backup failed. Your PC's files are untouched — only the USB drive was written to."
          detail={error}
          expanded
        />
      )}
      {warnings.length > 0 && (
        <div className="bg-amber-50 border border-amber-200 text-amber-900 rounded-lg p-4 mb-4 text-sm">
          <div className="font-medium mb-1">
            {succeeded
              ? "Completed with skipped extras — your files are safe:"
              : finished
                ? "Skipped extras (backup did NOT finish — files are not verified yet):"
                : "Extras skipped so far — backup still running, nothing verified yet:"}
          </div>
          <ul className="space-y-1">
            {warnings.map((w, i) => (
              <li key={i}>• {w}</li>
            ))}
          </ul>
        </div>
      )}
      <ProgressBar current={prog.current} total={prog.total} label={detail || "Working…"} />
      {!finished && (
        <p className="text-xs text-gray-500 mt-1">
          Elapsed {fmt(elapsed)} · last update {ago}s ago
          {ago > 30 ? " (slow step — still working)" : ""}
        </p>
      )}
      <StageList stages={STAGES.map((name, i) => ({ name, state: states[i] }))} />
      <p className="text-xs text-gray-500 mt-4">
        Do not remove the USB drive until this finishes.
      </p>
    </div>
  );
}
