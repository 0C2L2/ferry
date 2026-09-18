import { useState } from "react";
import { clearHistory, loadHistory, type HistoryRecord } from "../history";
import { formatBytes } from "../types";

function describe(r: HistoryRecord): string {
  if (r.kind === "backup") {
    return `${r.files.toLocaleString()} files · ${formatBytes(r.bytes)} → ${r.drive} (${r.os})`;
  }
  return `${r.restored.toLocaleString()} restored · ${r.skipped} skipped · ${r.wifi} Wi-Fi`;
}

export function MyFiles() {
  const [records, setRecords] = useState<HistoryRecord[]>(loadHistory);
  const [armingClear, setArmingClear] = useState(false);

  if (records.length === 0) {
    return (
      <div className="max-w-2xl text-center py-6">
        <p className="text-gray-500 text-sm">
          No backups or restores recorded on this PC yet. Your history appears here after a
          backup or restore run. File contents are never stored — only counts and dates.
        </p>
      </div>
    );
  }

  return (
    <div className="max-w-2xl">
      <div className="flex items-center justify-end mb-4">
        {!armingClear ? (
          <button
            onClick={() => setArmingClear(true)}
            className="text-sm text-gray-500 hover:text-red-600"
          >
            Clear history
          </button>
        ) : (
          <div className="flex gap-2 items-center text-sm">
            <span className="text-gray-600">Clear all history?</span>
            <button
              onClick={() => {
                clearHistory();
                setRecords([]);
                setArmingClear(false);
              }}
              className="px-3 py-1 rounded-lg bg-red-600 text-white hover:bg-red-700"
            >
              Yes, clear
            </button>
            <button
              onClick={() => setArmingClear(false)}
              className="px-3 py-1 rounded-lg border border-gray-300"
            >
              Cancel
            </button>
          </div>
        )}
      </div>
      <ul className="space-y-2">
        {records.map((r, i) => (
          <li
            key={`${r.date}-${i}`}
            className="bg-white border border-gray-200 rounded-xl px-4 py-3 flex items-center justify-between gap-3"
          >
            <div>
              <span
                className={`text-xs px-2 py-0.5 rounded-full mr-2 ${
                  r.kind === "backup"
                    ? "bg-brand-50 text-brand-700"
                    : "bg-green-100 text-green-800"
                }`}
              >
                {r.kind === "backup" ? "Backup" : "Restore"}
              </span>
              <span className="text-sm">{describe(r)}</span>
            </div>
            <span className="text-xs text-gray-400 shrink-0">
              {new Date(r.date).toLocaleString()}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
