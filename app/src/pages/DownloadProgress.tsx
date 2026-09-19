import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useTauriProgress } from "../hooks/useTauriProgress";
import { ProgressBar } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { OsSource, ProgressPayload } from "../types";

interface Props {
  dataRoot: string;
  os: OsSource;
  onDone: (isoFilename: string) => void;
}

export function DownloadProgress({ dataRoot, os, onDone }: Props) {
  const [prog, setProg] = useState({ current: 0, total: 0 });
  const [item, setItem] = useState("Starting download…");
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const [isoFilename, setIsoFilename] = useState<string | null>(null);
  const started = useRef(false);

  useTauriProgress("download:progress", (p: ProgressPayload) => {
    setProg({ current: p.current, total: p.total });
    setItem(p.current_item);
  });

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    api
      .downloadOs(os.id, dataRoot)
      .then(path => {
        const filename = path.split(/[\\/]/).pop() ?? path;
        setIsoFilename(filename);
        setDone(true);
      })
      .catch(err => setError(String(err)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pct =
    prog.total > 0 ? Math.round((prog.current / prog.total) * 100) : 0;

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Downloading {os.label}</h2>
      <p className="text-sm text-gray-500 mb-4">
        From the vendor's official server. The checksum is verified before use.
      </p>
      {error && <ErrorBox message="Download failed. Any partial file was deleted." detail={error} />}
      {!error && (
        <ProgressBar current={prog.current} total={prog.total} label={`${item} · ${pct}%`} />
      )}
      {done && isoFilename && (
        <button
          onClick={() => onDone(isoFilename)}
          className="mt-6 px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium hover:bg-brand-700"
        >
          Continue →
        </button>
      )}
    </div>
  );
}
