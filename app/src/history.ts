// Lightweight backup/restore journal (this PC only). Only counts and dates
// are stored — never file contents, names beyond counts, or passwords.

export type HistoryRecord =
  | {
      kind: "backup";
      date: string;
      drive: string;
      os: string;
      files: number;
      bytes: number;
    }
  | {
      kind: "restore";
      date: string;
      restored: number;
      skipped: number;
      wifi: number;
    };

const HISTORY_KEY = "ferry.history";
const MAX_RECORDS = 50;

export function loadHistory(): HistoryRecord[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? (parsed as HistoryRecord[]) : [];
  } catch {
    return [];
  }
}

function append(record: HistoryRecord) {
  const next = [record, ...loadHistory()].slice(0, MAX_RECORDS);
  localStorage.setItem(HISTORY_KEY, JSON.stringify(next));
}

export function recordBackup(input: {
  drive: string;
  os: string;
  files: number;
  bytes: number;
}) {
  append({ kind: "backup", date: new Date().toISOString(), ...input });
}

export function recordRestore(input: { restored: number; skipped: number; wifi: number }) {
  append({ kind: "restore", date: new Date().toISOString(), ...input });
}

export function clearHistory() {
  localStorage.removeItem(HISTORY_KEY);
}
