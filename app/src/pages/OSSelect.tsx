import { useEffect, useState } from "react";
import { api } from "../api";
import { formatBytes, type OsSource } from "../types";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  selected: OsSource | null;
  onSelect: (s: OsSource) => void;
  onNext: () => void;
}

export function OSSelect({ selected, onSelect, onNext }: Props) {
  const [sources, setSources] = useState<OsSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .listOsSources()
      .then(s => setSources(Array.isArray(s) ? s : []))
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Choose operating system</h2>
      <p className="text-sm text-gray-500 mb-4">
        Always downloaded from the vendor's own official servers — never re-hosted.
      </p>
      {error && <ErrorBox message="Could not load OS sources." detail={error} />}
      {loading ? (
        <p className="text-gray-500 animate-pulse">Loading OS options...</p>
      ) : (
        <ul className="space-y-3">
          {sources.map(s => {
            const active = selected?.id === s.id;
            const unavailable = s.source_type === "microsoft_mct" && !s.url;
            return (
              <li key={s.id}>
                <button
                  onClick={() => !unavailable && onSelect(s)}
                  disabled={unavailable}
                  aria-pressed={active}
                  className={`w-full text-left p-4 rounded-xl border bg-white transition ${
                    unavailable
                      ? "opacity-50 cursor-not-allowed border-gray-200"
                      : active
                        ? "border-brand-600 bg-brand-50 shadow"
                        : "border-gray-200 hover:border-brand-300"
                  }`}
                >
                  <div className="font-medium">{s.label}</div>
                  <div className="text-sm text-gray-500">
                    {s.approx_bytes ? `≈ ${formatBytes(s.approx_bytes)} download` : ""}
                    {unavailable
                      ? " · automated download not available in this build"
                      : s.source_type === "direct_url"
                        ? " · direct official download, checksum verified"
                        : " · via Microsoft's official tool"}
                  </div>
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
