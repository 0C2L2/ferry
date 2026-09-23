import { useEffect, useState } from "react";
import { ChevronDown, RefreshCw } from "lucide-react";
import { api } from "../api";
import { formatBytes, formatGB, type DriveInfo } from "../types";
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
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

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
        Only removable drives are listed. System drives can never be selected.
        Preparing a drive erases the <span className="font-medium">whole disk</span> —
        every partition on it — before anything is backed up. Nothing on your PC
        itself is touched.
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
            const isOpen = expanded.has(d.drive_letter);
            const toggle = () =>
              setExpanded(prev => {
                const next = new Set(prev);
                if (next.has(d.drive_letter)) next.delete(d.drive_letter);
                else next.add(d.drive_letter);
                return next;
              });
            return (
              <li
                key={d.drive_letter}
                className={`rounded-xl border bg-white transition ${
                  active ? "border-brand-600 bg-brand-50 shadow" : "border-gray-200"
                }`}
              >
                <button
                  onClick={() => onSelect(d)}
                  aria-pressed={active}
                  className="w-full flex items-center justify-between p-4 text-left hover:border-brand-300 rounded-xl transition"
                >
                  <div>
                    <div className="font-medium">
                      {d.drive_letter} · {d.model}
                    </div>
                    <div className="text-sm text-gray-500">
                      {formatBytes(d.total_bytes)} drive · {formatGB(d.free_bytes)} · removable
                    </div>
                    {d.partition_count > 1 && (
                      <div className="text-sm text-amber-700 mt-0.5">
                        Already has {d.partition_count} partitions — all of them will be erased
                      </div>
                    )}
                  </div>
                  {active && <span className="text-brand-700 font-bold">✓</span>}
                </button>
                {d.partitions.length > 1 && (
                  <>
                    <button
                      onClick={toggle}
                      aria-expanded={isOpen}
                      className="w-full flex items-center gap-1 px-4 pb-1 pt-0 text-xs text-gray-500 hover:text-gray-900"
                    >
                      <ChevronDown
                        className={`w-3.5 h-3.5 transition-transform ${isOpen ? "rotate-180" : ""}`}
                      />
                      {isOpen ? "Hide" : "Show"} {d.partitions.length} partitions
                    </button>
                    {isOpen && (
                      <ul className="mx-4 mb-3 rounded-lg bg-gray-50 border border-gray-100 divide-y divide-gray-100">
                        {d.partitions.map(p => (
                          <li
                            key={p.letter}
                            className="flex items-center justify-between px-3 py-2 text-sm"
                          >
                            <span>
                              <span className="font-medium">{p.letter}</span>{" "}
                              {p.label || <span className="text-gray-400">no label</span>}{" "}
                              <span className="text-gray-400">· {p.filesystem || "unknown fs"}</span>
                            </span>
                            <span className="text-gray-500 shrink-0 ml-3">
                              {formatBytes(p.total_bytes)} · {formatGB(p.free_bytes)}
                            </span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </>
                )}
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
