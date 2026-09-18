import { useState } from "react";
import { useAuth } from "../auth";
import { ErrorBox } from "../components/ErrorBox";

export function Account() {
  const { account, signUp, signIn, signOut } = useAuth();
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

  if (account) {
    return (
      <div className="max-w-xl mx-auto">
        <h2 className="text-xl font-semibold mb-4">Your account</h2>
        <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
          <div className="font-medium text-lg">{account.name}</div>
          <div className="text-sm text-gray-500">{account.email}</div>
          <div className="mt-3 text-sm">
            Plan: <span className="font-medium">Ferry Free</span>
            <span className="text-gray-500"> — core backup & USB tools, free forever.</span>
          </div>
        </div>
        <p className="text-sm text-gray-500 mb-4">
          This account lives only on this PC in this build. When Cloud Backup launches, it will
          carry over as your cloud identity.
        </p>
        <button
          onClick={signOut}
          className="px-5 py-2 rounded-lg border border-gray-300 text-sm hover:border-brand-600"
        >
          Sign out
        </button>
      </div>
    );
  }

  return (
    <div className="max-w-md mx-auto">
      <h2 className="text-xl font-semibold mb-1">{mode === "up" ? "Create account" : "Sign in"}</h2>
      <p className="text-sm text-gray-500 mb-4">
        Optional — the core app never requires an account. One is needed only for the planned
        Cloud Backup feature.
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
