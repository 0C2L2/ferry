import { useEffect, useState } from "react";
import { TriangleAlert, ArrowRight } from "lucide-react";
import { api } from "../api";
import { formatBytes, type MigrationProfile, type OsSource } from "../types";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  os: OsSource;
  onDone: (roots: string[]) => void;
}

export function MigrationPlan({ os, onDone }: Props) {
  const [profile, setProfile] = useState<MigrationProfile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [checked, setChecked] = useState<Set<string>>(new Set());

  useEffect(() => {
    api
      .migrationProfile(os.id)
      .then(p => {
        setProfile(p);
        setChecked(new Set(p.folders.filter(f => f.recommended).map(f => f.path)));
      })
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, [os.id]);

  function toggle(path: string) {
    setChecked(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Plan your migration</h2>
      {profile && (
        <p className="text-sm text-gray-500 mb-4 flex items-center gap-2 flex-wrap">
          <span className="font-medium text-gray-700">{profile.source_os}</span>
          <ArrowRight className="w-4 h-4" />
          <span className="font-medium text-gray-700">{os.label}</span>
        </p>
      )}
      {error && <ErrorBox message="Could not plan migration." detail={error} />}
      {loading || !profile ? (
        !error && <p className="text-gray-500 animate-pulse">Measuring your folders…</p>
      ) : (
        <>
          {profile.warnings.length > 0 && (
            <div className="bg-amber-50 border border-amber-200 text-amber-900 rounded-lg p-4 mb-4 text-sm space-y-1.5">
              <div className="flex items-center gap-2 font-medium">
                <TriangleAlert className="w-4 h-4 shrink-0" /> Before you choose
              </div>
              {profile.warnings.map(w => (
                <p key={w}>• {w}</p>
              ))}
            </div>
          )}
          <div className="flex gap-2 mb-3 text-sm">
            <button
              onClick={() =>
                setChecked(new Set(profile.folders.filter(f => f.recommended).map(f => f.path)))
              }
              className="px-3 py-1.5 rounded-lg border border-gray-300 hover:border-brand-600"
            >
              Recommended only
            </button>
            <button
              onClick={() => setChecked(new Set(profile.folders.map(f => f.path)))}
              className="px-3 py-1.5 rounded-lg border border-gray-300 hover:border-brand-600"
            >
              Select all
            </button>
            <button
              onClick={() => setChecked(new Set())}
              className="px-3 py-1.5 rounded-lg border border-gray-300 hover:border-brand-600"
            >
              Clear
            </button>
          </div>
          <ul className="space-y-2 mb-6">
            {profile.folders.map(f => {
              const on = checked.has(f.path);
              return (
                <li key={f.path}>
                  <label
                    className={`flex items-center justify-between gap-3 p-3 rounded-xl border bg-white cursor-pointer transition ${
                      on ? "border-brand-600 bg-brand-50" : "border-gray-200 hover:border-brand-300"
                    }`}
                  >
                    <span className="flex items-center gap-3 min-w-0">
                      <input
                        type="checkbox"
                        checked={on}
                        onChange={() => toggle(f.path)}
                        className="shrink-0"
                      />
                      <span className="min-w-0">
                        <span className="font-medium block truncate">{f.name}</span>
                        <span className="text-xs text-gray-500">
                          {f.file_count.toLocaleString()} files
                          {f.recommended ? " · recommended" : ""}
                        </span>
                      </span>
                    </span>
                    <span className="text-sm text-gray-500 shrink-0">{formatBytes(f.size_bytes)}</span>
                  </label>
                </li>
              );
            })}
          </ul>
          <button
            onClick={() => onDone([...checked])}
            disabled={checked.size === 0}
            className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
          >
            Back up {checked.size} folder{checked.size === 1 ? "" : "s"} →
          </button>
        </>
      )}
    </div>
  );
}
