import { useEffect, useState } from "react";
import { api } from "../api";
import { formatBytes, type ScanResult } from "../types";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  scan: ScanResult | null;
  extraExcludes: string[];
  roots: string[];
  onScan: (scan: ScanResult, extraExcludes: string[]) => void;
  onNext: () => void;
}

export function ExcludeReview({ scan, extraExcludes, roots, onScan, onNext }: Props) {
  const [loading, setLoading] = useState(!scan);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [pending, setPending] = useState<string[]>(extraExcludes);

  async function runScan(excludes: string[]) {
    setLoading(true);
    setError(null);
    try {
      const result = await api.scanFiles(excludes, roots);
      onScan(result, excludes);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (!scan) void runScan(pending);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Review what gets backed up</h2>
      <p className="text-sm text-gray-500 mb-4">
        Only your selected folders are scanned — everything else on this PC is left alone.
        OS caches and temp files inside them are skipped by default.
      </p>
      {error && <ErrorBox message="Backup scan failed." detail={error} />}
      {loading || !scan ? (
        <p className="text-gray-500 animate-pulse">Scanning your files...</p>
      ) : (
        <>
          <div className="bg-white border border-gray-200 rounded-xl p-4 mb-4 text-sm">
            <div>
              <span className="font-medium">{scan.files.length.toLocaleString()} files</span>
              {" · "}
              {formatBytes(scan.total_bytes)}
            </div>
            <div className="text-gray-500">
              {scan.skipped_count.toLocaleString()} items skipped (caches, temp files)
            </div>
          </div>
          <div className="flex gap-2 mb-4">
            <input
              value={draft}
              onChange={e => setDraft(e.target.value)}
              placeholder="Extra folder to skip, e.g. Videos\Raw"
              className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm"
            />
            <button
              onClick={() => {
                if (draft.trim()) {
                  const next = [...pending, draft.trim()];
                  setPending(next);
                  setDraft("");
                  void runScan(next);
                }
              }}
              className="px-4 py-2 rounded-lg border border-gray-300 text-sm hover:border-brand-600"
            >
              Add & rescan
            </button>
          </div>
          {pending.length > 0 && (
            <ul className="text-sm text-gray-600 mb-4 space-y-1">
              {pending.map(p => (
                <li key={p} className="flex justify-between bg-gray-100 rounded px-3 py-1">
                  <span>{p}</span>
                  <button
                    className="text-red-600"
                    onClick={() => {
                      const next = pending.filter(x => x !== p);
                      setPending(next);
                      void runScan(next);
                    }}
                  >
                    remove
                  </button>
                </li>
              ))}
            </ul>
          )}
          <button
            onClick={onNext}
            className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium hover:bg-brand-700"
          >
            Continue →
          </button>
        </>
      )}
    </div>
  );
}
