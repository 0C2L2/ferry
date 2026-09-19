import { Check } from "lucide-react";

// Ferry is free — all of it, including Cloud Backup. No tiers, no
// subscription, no account required for any feature. See
// company/business-model.md for the reasoning.

const FEATURES = [
  "Personal-file backup to USB",
  "Official OS downloads",
  "Bootable USB creation",
  "Encrypted backup & restore",
  "App reinstall checklist",
  "Cloud Backup (bring your own free-tier storage — Ferry never charges for it)",
  "No account required",
];

export function Pricing({ onAccount }: { onAccount: () => void }) {
  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="text-2xl font-bold text-center mb-2">Pricing</h2>
      <p className="text-gray-600 text-center mb-8">Ferry is free. All of it.</p>

      <div className="rounded-xl border-2 border-brand-600 bg-white p-6 shadow-sm">
        <div className="font-semibold text-lg">Ferry</div>
        <div className="text-3xl font-bold my-2">
          $0 <span className="text-base font-normal text-gray-500">forever</span>
        </div>
        <ul className="text-sm space-y-2 mt-4">
          {FEATURES.map(f => (
            <li key={f} className="flex gap-2">
              <Check className="w-4 h-4 text-green-600 shrink-0 mt-0.5" /> {f}
            </li>
          ))}
        </ul>
        <button
          onClick={onAccount}
          className="mt-5 text-sm text-brand-700 hover:text-brand-600"
        >
          Create an optional local profile →
        </button>
      </div>

      <p className="text-xs text-gray-500 text-center mt-6">
        Two promises: your files are never monetized, and there is no payment path that could
        ever influence which download source the app picker recommends.
      </p>
    </div>
  );
}
