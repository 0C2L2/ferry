import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { api } from "../api";
import { formatGB, type DriveInfo } from "../types";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  selected: DriveInfo | null;
  onSelect: (d: DriveInfo) => void;
  onNext: () => void;
}

export function DriveSelect({ selected, onSelect, onNext }: Props) {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const result = await api.listDrives();
      setDrives(Array.isArray(result) ? result : []);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-2xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">Select target USB drive</h2>
        <button
          onClick={refresh}
          className="flex items-center gap-1 text-sm text-brand-700 hover:text-brand-600"
        >
          <RefreshCw className="w-4 h-4" /> Rescan
        </button>
      </div>
      <p className="text-sm text-gray-500 mb-4">
        Only removable drives are listed. System drives can never be selected. Everything on
        the chosen drive will be erased later — after your backup is verified.
      </p>
      {error && <ErrorBox message="Could not list USB drives." detail={error} />}
      {loading ? (
        <p className="text-gray-500 animate-pulse">Scanning for USB drives...</p>
      ) : drives.length === 0 ? (
        <p className="text-gray-500">No removable USB drives found. Please insert a drive.</p>
      ) : (
        <ul className="space-y-3">
          {drives.map(d => {
            const active = selected?.drive_letter === d.drive_letter;
            return (
              <li key={d.drive_letter}>
                <button
                  onClick={() => onSelect(d)}
                  aria-pressed={active}
                  className={`w-full flex items-center justify-between p-4 rounded-xl border bg-white transition ${
                    active
                      ? "border-brand-600 bg-brand-50 shadow"
                      : "border-gray-200 hover:border-brand-300"
                  }`}
                >
                  <div className="text-left">
                    <div className="font-medium">
                      {d.drive_letter} · {d.model}
                    </div>
                    <div className="text-sm text-gray-500">
                      {formatGB(d.free_bytes)} · removable
                    </div>
                  </div>
                  {active && <span className="text-brand-700 font-bold">✓</span>}
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <button
        onClick={onNext}
        disabled={!selected}
        className="mt-6 px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
      >
        Continue →
      </button>
    </div>
  );
}
