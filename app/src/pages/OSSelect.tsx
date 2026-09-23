import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import { api } from "../api";
import { formatBytes, type CustomIso, type OsSource } from "../types";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  selected: OsSource | null;
  custom: CustomIso | null;
  onSelect: (s: OsSource | null) => void;
  onCustom: (c: CustomIso | null) => void;
  onNext: () => void;
}

interface DragPayload {
  paths: string[];
}

export function OSSelect({ selected, custom, onSelect, onCustom, onNext }: Props) {
  const [sources, setSources] = useState<OsSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [customBusy, setCustomBusy] = useState(false);

  useEffect(() => {
    api
      .listOsSources()
      .then(s => setSources(Array.isArray(s) ? s : []))
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, []);

  // Native OS file drop onto the window (Tauri drag-drop event). The HTML5
  // drop zone below covers in-window drags; this covers drops from Explorer.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<DragPayload>("tauri://drag-drop", e => {
      const first = e.payload.paths?.[0];
      if (first) void acceptFile(first);
    }).then(u => {
      unlisten = u;
    });
    return () => {
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function acceptFile(path: string) {
    setCustomBusy(true);
    setError(null);
    try {
      const info = await api.inspectCustomIso(path);
      onCustom({ path, filename: info.filename, size_bytes: info.size_bytes });
      onSelect({
        id: "custom-iso",
        label: info.filename,
        source_type: "direct_url",
        url: null,
        checksum_url: null,
        approx_bytes: info.size_bytes,
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setCustomBusy(false);
    }
  }

  async function browse() {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "OS image", extensions: ["iso"] }],
      });
      if (typeof picked === "string" && picked) await acceptFile(picked);
    } catch (err) {
      setError(String(err));
    }
  }

  function clearCustom() {
    onCustom(null);
    onSelect(null);
  }

  const customActive = selected?.id === "custom-iso" && custom !== null;

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Choose operating system</h2>
      <p className="text-sm text-gray-500 mb-4">
        Official downloads come from the vendor's own servers — never re-hosted.
        Or bring your own <span className="font-medium">.iso</span> file.
      </p>
      {error && <ErrorBox message="Could not use that file." detail={error} />}
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

      <div
        onDragOver={e => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={e => {
          e.preventDefault();
          setDragOver(false);
          const f = e.dataTransfer.files?.[0] as unknown as { path?: string } | undefined;
          if (f?.path) void acceptFile(f.path);
        }}
        className={`mt-4 p-5 rounded-xl border-2 border-dashed bg-white transition text-center ${
          dragOver ? "border-brand-600 bg-brand-50" : "border-gray-300"
        }`}
      >
        {customActive && custom ? (
          <div className="text-left">
            <div className="font-medium">Custom image: {custom.filename}</div>
            <div className="text-sm text-gray-500">
              {formatBytes(custom.size_bytes)} · user-supplied, checksum cannot be verified
              against a vendor hash ·{" "}
              <button onClick={clearCustom} className="text-red-600 hover:text-red-700">
                remove
              </button>
            </div>
          </div>
        ) : (
          <>
            <FileUp className="w-6 h-6 mx-auto mb-2 text-gray-400" />
            <p className="text-sm text-gray-600">
              {customBusy ? "Checking file…" : "Drop an .iso file here, or"}{" "}
              {!customBusy && (
                <button onClick={browse} className="text-brand-700 hover:text-brand-600 font-medium">
                  browse your files
                </button>
              )}
            </p>
            <p className="text-xs text-gray-400 mt-1">
              User-supplied images skip the vendor checksum — only use files you trust.
            </p>
          </>
        )}
      </div>

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
