interface Props {
  current: number;
  total: number;
  label?: string;
}

/** Determinate bar; falls back to an indeterminate shimmer when total is 0. */
export function ProgressBar({ current, total, label }: Props) {
  const pct = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0;
  return (
    <div className="w-full">
      {label && (
        <div className="flex justify-between text-sm text-gray-600 mb-1">
          <span className="truncate">{label}</span>
          {total > 0 && (
            <span>
              {current}/{total} ({pct}%)
            </span>
          )}
        </div>
      )}
      <div className="h-2.5 rounded-full bg-gray-200 overflow-hidden">
        {total > 0 ? (
          <div
            className="h-full bg-brand-600 rounded-full transition-all"
            style={{ width: `${pct}%` }}
          />
        ) : (
          <div className="h-full w-1/3 bg-brand-300 rounded-full animate-pulse" />
        )}
      </div>
    </div>
  );
}

export type StageState = "pending" | "active" | "done" | "error";

export function StageList({ stages }: { stages: { name: string; state: StageState; detail?: string }[] }) {
  const icon = (s: StageState) =>
    s === "done" ? "✓" : s === "active" ? "…" : s === "error" ? "✕" : "○";
  const cls = (s: StageState) =>
    s === "done"
      ? "text-green-700"
      : s === "active"
        ? "text-brand-700 font-medium"
        : s === "error"
          ? "text-red-700"
          : "text-gray-400";
  return (
    <ul className="space-y-2 mt-4">
      {stages.map(s => (
        <li key={s.name} className={`flex items-start gap-2 text-sm ${cls(s.state)}`}>
          <span className="w-5 shrink-0">{icon(s.state)}</span>
          <span>
            {s.name}
            {s.detail && <span className="block text-xs text-gray-500">{s.detail}</span>}
          </span>
        </li>
      ))}
    </ul>
  );
}
