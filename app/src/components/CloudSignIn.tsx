import { useState } from "react";
import { api } from "../api";
import { ErrorBox } from "./ErrorBox";

interface Props {
  /** What signing in is for, e.g. "to back up to Ferry Cloud". */
  purpose: string;
  onSignedIn: (email: string) => void;
}

/** Passwordless sign-in: email → 6-digit code → signed in for this session. */
export function CloudSignIn({ purpose, onSignedIn }: Props) {
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [sent, setSent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function run(step: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await step();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const sendCode = () =>
    run(async () => {
      await api.cloudSignInStart(email.trim());
      setSent(true);
    });

  const verify = () =>
    run(async () => {
      onSignedIn(await api.cloudSignInVerify(email.trim(), code.trim()));
    });

  return (
    <div className="bg-white border border-gray-200 rounded-xl p-5 mb-4">
      <div className="font-medium mb-1">Sign in {purpose}</div>
      <p className="text-sm text-gray-500 mb-3">
        No password — we email you a 6-digit code. Your backup is tied to this address.
      </p>
      {error && <ErrorBox message="Sign-in didn't work." detail={error} />}
      {!sent ? (
        <div className="flex gap-2">
          <input
            type="email"
            value={email}
            onChange={e => setEmail(e.target.value)}
            onKeyDown={e => e.key === "Enter" && email.trim() && void sendCode()}
            placeholder="you@example.com"
            disabled={busy}
            className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm"
          />
          <button
            onClick={() => void sendCode()}
            disabled={busy || !email.trim()}
            className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium disabled:opacity-40 hover:bg-brand-700"
          >
            {busy ? "Sending…" : "Email me a code"}
          </button>
        </div>
      ) : (
        <>
          <p className="text-sm mb-2">
            Code sent to <strong>{email.trim()}</strong>. It's valid for 10 minutes.
          </p>
          <div className="flex gap-2">
            <input
              inputMode="numeric"
              maxLength={6}
              value={code}
              onChange={e => setCode(e.target.value.replace(/\D/g, ""))}
              onKeyDown={e => e.key === "Enter" && code.length === 6 && void verify()}
              placeholder="123456"
              disabled={busy}
              className="w-32 px-3 py-2 border border-gray-300 rounded-lg text-sm font-mono tracking-widest"
            />
            <button
              onClick={() => void verify()}
              disabled={busy || code.length !== 6}
              className="px-4 py-2 rounded-lg bg-brand-600 text-white text-sm font-medium disabled:opacity-40 hover:bg-brand-700"
            >
              {busy ? "Checking…" : "Sign in"}
            </button>
            <button
              onClick={() => {
                setSent(false);
                setCode("");
              }}
              disabled={busy}
              className="px-3 py-2 text-sm text-gray-500 hover:text-gray-900"
            >
              Use a different email
            </button>
          </div>
        </>
      )}
    </div>
  );
}
