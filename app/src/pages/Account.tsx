import { useState } from "react";
import { useAuth } from "../auth";
import { ErrorBox } from "../components/ErrorBox";
import { MyFiles } from "./MyFiles";
import { SettingsPage } from "./Settings";

interface Props {
  tab: AccountTab;
  onTabChange: (t: AccountTab) => void;
}

type Tab = "profile" | "plan" | "history" | "preferences";

export type AccountTab = Tab;

const TABS: { id: Tab; label: string }[] = [
  { id: "profile", label: "Profile" },
  { id: "plan", label: "Plan" },
  { id: "history", label: "History" },
  { id: "preferences", label: "Settings" },
];

export function Account({ tab, onTabChange }: Props) {
  const { account, plannedTier, setPlannedTier, signUp, signIn, signOut } = useAuth();
  const setTab = onTabChange;
  const [mode, setMode] = useState<"in" | "up">("in");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit() {
    setBusy(true);
    setError(null);
    const err = mode === "up" ? await signUp(name, email, password) : await signIn(email, password);
    setError(err);
    setBusy(false);
  }

  if (!account) {
    return (
      <div className="max-w-md mx-auto">
        <h2 className="text-xl font-semibold mb-1">{mode === "up" ? "Create account" : "Sign in"}</h2>
        <p className="text-sm text-gray-500 mb-4">
          Optional — every core Ferry feature works without an account. You'll
          register and sign in when you buy the Cloud Backup add-on.
        </p>
        {error && <ErrorBox message={error} />}
        {mode === "up" && (
          <>
            <label className="block text-sm font-medium mb-1">Name</label>
            <input
              value={name}
              onChange={e => setName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3"
            />
          </>
        )}
        <label className="block text-sm font-medium mb-1">Email</label>
        <input
          value={email}
          onChange={e => setEmail(e.target.value)}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3"
        />
        <label className="block text-sm font-medium mb-1">Password (min 8 characters)</label>
        <input
          type="password"
          value={password}
          onChange={e => setPassword(e.target.value)}
          onKeyDown={e => {
            if (e.key === "Enter") void submit();
          }}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-4"
        />
        <button
          onClick={() => void submit()}
          disabled={busy}
          className="w-full px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium disabled:opacity-40 hover:bg-brand-700"
        >
          {busy ? "…" : mode === "up" ? "Create account" : "Sign in"}
        </button>
        <button
          onClick={() => {
            setMode(mode === "up" ? "in" : "up");
            setError(null);
          }}
          className="mt-3 text-sm text-brand-700 hover:text-brand-600"
        >
          {mode === "up" ? "Already have one? Sign in" : "New here? Create account"}
        </button>
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Your account</h2>
      <div className="flex flex-col sm:flex-row gap-6">
        <div className="sm:w-44 shrink-0 flex sm:flex-col gap-1 overflow-x-auto">
          {TABS.map(t => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`px-3 py-2 rounded-lg text-sm text-left whitespace-nowrap transition ${
                tab === t.id
                  ? "bg-brand-50 text-brand-700 font-semibold"
                  : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>
        <div className="flex-1 min-w-0">

      {tab === "profile" && (
        <div className="max-w-xl">
          <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
            <div className="font-medium text-lg">{account.name}</div>
            <div className="text-sm text-gray-500">{account.email}</div>
            <div className="text-xs text-gray-400 mt-1">
              Member since {new Date(account.createdAt).toLocaleDateString()}
            </div>
          </div>
          <p className="text-sm text-gray-500 mb-4">
            This profile lives only on this PC and is entirely optional for the
            free core features — you'll sign in with it when you buy Cloud Backup.
          </p>
          <button
            onClick={signOut}
            className="px-5 py-2 rounded-lg border border-gray-300 text-sm hover:border-brand-600"
          >
            Sign out
          </button>
        </div>
      )}

      {tab === "plan" && (
        <div className="max-w-xl">
          <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
            <div className="text-sm text-gray-500">Current plan</div>
            <div className="font-semibold text-lg">Ferry Free</div>
            <p className="text-sm text-gray-500 mt-1">
              Core backup & USB tools, free forever. No subscription, no card.
            </p>
          </div>
          <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
            <div className="text-sm text-gray-500">Cloud Backup (paid add-on)</div>
            {plannedTier ? (
              <>
                <div className="font-medium">
                  {plannedTier} <span className="text-brand-700">✓ planned</span>
                </div>
                <p className="text-sm text-gray-500 mt-1">
                  Saved preference — shown again at checkout when billing launches.
                </p>
                <button
                  onClick={() => setPlannedTier(null)}
                  className="mt-3 text-sm text-red-600 hover:text-red-700"
                >
                  Remove planned tier
                </button>
              </>
            ) : (
              <>
                <p className="text-sm text-gray-500 mt-1">No tier selected.</p>
                <p className="text-sm text-gray-500 mt-1">
                  One-time per migration: ~$5 / 50 GB, ~$15 / 200 GB, ~$40 / 1 TB.
                </p>
              </>
            )}
          </div>
        </div>
      )}

      {tab === "history" && <MyFiles />}

      {tab === "preferences" && <SettingsPage />}
        </div>
      </div>
    </div>
  );
}
