import { useState } from "react";
import { TriangleAlert } from "lucide-react";
import { ErrorBox } from "../components/ErrorBox";

interface Props {
  onNext: (password: string) => void;
}

export function PasswordSetup({ onNext }: Props) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [writtenDown, setWrittenDown] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function submit() {
    if (password.trim().length < 12) {
      setError("Use at least 12 characters — this password protects your entire backup.");
      return;
    }
    if (password !== confirm) {
      setError("Passwords do not match.");
      return;
    }
    if (!writtenDown) {
      setError("Please confirm you wrote the password down before continuing.");
      return;
    }
    setError(null);
    onNext(password);
  }

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-1">Set backup password</h2>
      <p className="text-sm text-gray-500 mb-4">
        Your backup includes browser passwords and Wi-Fi keys, so encryption is mandatory.
      </p>
      <div className="bg-amber-50 border border-amber-200 text-amber-900 rounded-lg p-4 mb-4 flex gap-2 text-sm">
        <TriangleAlert className="w-5 h-5 shrink-0" />
        <p>
          Write this password down somewhere safe. <strong>We cannot recover it for you</strong>
          {" — "}losing it means losing the backup.
        </p>
      </div>
      {error && <ErrorBox message={error} />}
      <label className="block text-sm font-medium mb-1">Password (min 12 characters)</label>
      <input
        type="password"
        value={password}
        onChange={e => setPassword(e.target.value)}
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-3"
      />
      <label className="block text-sm font-medium mb-1">Confirm password</label>
      <input
        type="password"
        value={confirm}
        onChange={e => setConfirm(e.target.value)}
        className="w-full px-3 py-2 border border-gray-300 rounded-lg mb-4"
      />
      <label className="flex items-start gap-2 text-sm mb-6">
        <input
          type="checkbox"
          checked={writtenDown}
          onChange={e => setWrittenDown(e.target.checked)}
          className="mt-1"
        />
        I have written this password down somewhere safe.
      </label>
      <button
        onClick={submit}
        className="px-6 py-2.5 rounded-lg bg-brand-600 text-white font-medium hover:bg-brand-700"
      >
        Start backup →
      </button>
    </div>
  );
}
