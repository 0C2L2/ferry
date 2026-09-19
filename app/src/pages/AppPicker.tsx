import { useEffect, useState } from "react";
import { api, assistApi, isAssistConfigured } from "../api";
import { ErrorBox } from "../components/ErrorBox";
import type { AppEntry, AppSuggestion, DriverEntry, RestoreSummary } from "../types";

interface Props {
  stagingDir: string | null;
  summary: RestoreSummary | null;
  onRestart: () => void;
}

function TierBadge({ tier }: { tier: number }) {
  const style =
    tier === 1
      ? "bg-green-100 text-green-800"
      : tier === 2
        ? "bg-amber-100 text-amber-800"
        : "bg-gray-200 text-gray-600";
  const label = tier === 1 ? "Tier 1 · winget" : tier === 2 ? "Tier 2 · vendor site" : "Tier 3 · no source";
  return <span className={`text-xs px-2 py-0.5 rounded-full ${style}`}>{label}</span>;
}

export function AppPicker({ stagingDir, summary, onRestart }: Props) {
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [drivers, setDrivers] = useState<DriverEntry[]>([]);
  const [sourceOs, setSourceOs] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [assistOn, setAssistOn] = useState(false);
  const [assistLoading, setAssistLoading] = useState(false);
  const [assistError, setAssistError] = useState<string | null>(null);
  const [suggestions, setSuggestions] = useState<Record<string, AppSuggestion>>({});
  const [verifying, setVerifying] = useState<string | null>(null);
  const [verified, setVerified] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (!stagingDir) {
      setLoading(false);
      return;
    }
    api
      .readInventory(stagingDir)
      .then(inv => {
        setApps(Array.isArray(inv.apps) ? inv.apps : []);
        setDrivers(Array.isArray(inv.drivers) ? inv.drivers : []);
      })
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
    api
      .readManifest(stagingDir)
      .then(m => setSourceOs(m.source_os))
      .catch(() => setSourceOs(null));
  }, [stagingDir]);

  async function install(app: AppEntry) {
    if (!app.winget_id) return;
    setInstalling(app.winget_id);
    setNotice(null);
    try {
      const msg = await api.installApp(app.winget_id);
      setNotice(msg);
    } catch (err) {
      setNotice(`Failed: ${String(err)}`);
    } finally {
      setInstalling(null);
    }
  }

  const shown = apps.filter(a => a.name.toLowerCase().includes(filter.toLowerCase()));
  const unresolved = apps.filter(a => a.tier !== 1);

  async function getAssistSuggestions() {
    setAssistLoading(true);
    setAssistError(null);
    try {
      const { suggestions: results } = await assistApi.suggestForApps(
        unresolved.map(a => ({ name: a.name, publisher: a.publisher, tier: a.tier })),
        "windows",
      );
      const byName: Record<string, AppSuggestion> = {};
      for (const s of results) byName[s.name] = s;
      setSuggestions(byName);
    } catch (err) {
      setAssistError(String(err));
    } finally {
      setAssistLoading(false);
    }
  }

  async function verifyCommand(appName: string, command: string) {
    setVerifying(appName);
    try {
      const result = await assistApi.verifyCommand(command);
      setVerified(v => ({ ...v, [appName]: result.success }));
    } catch (err) {
      setAssistError(String(err));
    } finally {
      setVerifying(null);
    }
  }

  return (
    <div className="max-w-3xl mx-auto">
      {summary && (
        <div className="bg-green-50 border border-green-200 text-green-900 rounded-xl p-4 mb-6 text-sm">
          Restored <strong>{summary.restored}</strong> files
          {summary.skipped > 0 && `, skipped ${summary.skipped}`}
          {summary.failed.length > 0 && ` (${summary.failed.length} issues — see details below)`}
          {` · Wi-Fi: ${summary.wifi_restored} reconnected, ${summary.wifi_failed} failed`}.
        </div>
      )}
      <h2 className="text-xl font-semibold mb-1">Reinstall your apps</h2>
      <p className="text-sm text-gray-500 mb-4">
        From your pre-wipe inventory. Apps are never restored as binaries — only reinstalled
        from verified sources.
      </p>
      {sourceOs && (
        <div className="bg-amber-50 border border-amber-200 text-amber-900 rounded-lg p-3 mb-4 text-sm">
          This backup came from <strong>{sourceOs}</strong>. Reinstall the same edition —
          a Windows license will not activate on a different edition.
        </div>
      )}
      {error && <ErrorBox message="Could not load the app inventory." detail={error} />}
      {notice && <div className="bg-brand-50 text-brand-700 rounded-lg p-3 mb-4 text-sm">{notice}</div>}
      {isAssistConfigured() && unresolved.length > 0 && (
        <div className="bg-violet-50 border border-violet-200 rounded-lg p-3 mb-4 text-sm">
          <label className="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" checked={assistOn} onChange={e => setAssistOn(e.target.checked)} />
            <span className="font-medium text-violet-900">
              Ferry Assist (beta) — get AI help for the {unresolved.length} app
              {unresolved.length === 1 ? "" : "s"} without a confident source
            </span>
          </label>
          <p className="text-xs text-violet-700 mt-1">
            Only app names/publishers are sent, never your files. Any suggested command is
            dry-run in an isolated cloud sandbox before it's ever shown as safe — nothing
            untested touches this PC.
          </p>
          {assistOn && (
            <button
              onClick={getAssistSuggestions}
              disabled={assistLoading}
              className="mt-2 text-xs px-3 py-1.5 rounded-lg bg-violet-600 text-white disabled:opacity-40 hover:bg-violet-700"
            >
              {assistLoading ? "Asking Ferry Assist…" : "Get AI suggestions"}
            </button>
          )}
          {assistError && <p className="text-xs text-red-600 mt-2">{assistError}</p>}
        </div>
      )}
      <input
        value={filter}
        onChange={e => setFilter(e.target.value)}
        placeholder="Filter apps…"
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3 text-sm"
      />
      {loading ? (
        <p className="text-gray-500 animate-pulse">Loading inventory…</p>
      ) : (
        <ul className="space-y-2 max-h-96 overflow-y-auto pr-1">
          {shown.map(a => {
            const suggestion = suggestions[a.name];
            return (
              <li key={a.name} className="bg-white border border-gray-200 rounded-lg px-4 py-2.5">
                <div className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div className="font-medium truncate">{a.name}</div>
                    <div className="text-xs text-gray-500">
                      {[a.version, a.publisher].filter(Boolean).join(" · ")}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <TierBadge tier={a.tier} />
                    {a.tier === 1 && a.winget_id ? (
                      <button
                        onClick={() => install(a)}
                        disabled={installing === a.winget_id}
                        className="text-xs px-3 py-1.5 rounded-lg bg-brand-600 text-white disabled:opacity-40 hover:bg-brand-700"
                      >
                        {installing === a.winget_id ? "Installing…" : "Install"}
                      </button>
                    ) : a.tier === 2 && a.url_info ? (
                      <span className="text-xs text-gray-500">Get from vendor site (unverified link)</span>
                    ) : (
                      <span className="text-xs text-gray-400">No confident source</span>
                    )}
                  </div>
                </div>
                {suggestion && (
                  <div className="mt-2 pt-2 border-t border-gray-100 text-xs text-gray-600">
                    <p>🤖 {suggestion.suggestion}</p>
                    {suggestion.command && (
                      <div className="mt-1.5 flex items-center gap-2 flex-wrap">
                        <code className="bg-gray-100 px-2 py-0.5 rounded">{suggestion.command}</code>
                        {verified[a.name] ? (
                          <span className="text-green-700">✓ Verified in sandbox</span>
                        ) : (
                          <button
                            onClick={() => verifyCommand(a.name, suggestion.command!)}
                            disabled={verifying === a.name}
                            className="px-2 py-0.5 rounded bg-gray-200 hover:bg-gray-300 disabled:opacity-40"
                          >
                            {verifying === a.name ? "Testing in sandbox…" : "Verify in sandbox"}
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
      {drivers.length > 0 && (
        <details className="mt-6 text-sm">
          <summary className="cursor-pointer font-medium">
            Driver list ({drivers.length}) — informational
          </summary>
          <ul className="mt-2 space-y-1 text-gray-600 max-h-48 overflow-y-auto">
            {drivers.slice(0, 200).map((d, i) => (
              <li key={`${d.name}-${i}`}>
                {d.name}
                {d.third_party ? " · third-party" : ""}
                {d.version ? ` · ${d.version}` : ""}
              </li>
            ))}
          </ul>
        </details>
      )}
      <button onClick={onRestart} className="mt-6 text-brand-700 hover:text-brand-600 text-sm">
        ← Back to start
      </button>
    </div>
  );
}
