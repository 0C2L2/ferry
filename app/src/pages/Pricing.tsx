import { Check } from "lucide-react";
import { useAuth } from "../auth";

// Cloud Backup is free for now (server flag CLOUD_FREE — no payment provider
// yet). The paid tiers in company/business-model.md return when payments do.

export function Pricing({ onAccount }: { onAccount: () => void }) {
  const { account, plannedTier, setPlannedTier } = useAuth();
  const corporateChosen = plannedTier === "Corporate";

  return (
    <div className="max-w-4xl mx-auto">
      <h2 className="text-2xl font-bold text-center mb-2">Pricing</h2>
      <p className="text-gray-600 text-center mb-8">
        The tools are free. You only ever pay for cloud storage during a migration.
      </p>

      <div className="grid md:grid-cols-2 gap-4 mb-8">
        <div className="rounded-xl border-2 border-brand-600 bg-white p-6 shadow-sm">
          <div className="font-semibold text-lg">Ferry Free</div>
          <div className="text-3xl font-bold my-2">
            $0 <span className="text-base font-normal text-gray-500">forever</span>
          </div>
          <ul className="text-sm space-y-2 mt-4">
            {[
              "Personal-file backup to USB",
              "Official OS downloads",
              "Bootable USB creation",
              "Encrypted backup & restore",
              "App reinstall checklist",
              "No account required",
            ].map(f => (
              <li key={f} className="flex gap-2">
                <Check className="w-4 h-4 text-green-600 shrink-0 mt-0.5" /> {f}
              </li>
            ))}
          </ul>
          <div className="mt-5 text-sm font-medium text-brand-700">
            {account ? `Active for ${account.name}` : "Active — no account needed"}
          </div>
        </div>

        <div className="rounded-xl border border-gray-200 bg-white p-6">
          <div className="font-semibold text-lg">Cloud Backup</div>
          <div className="text-3xl font-bold my-2">
            $0 <span className="text-base font-normal text-gray-500">for now</span>
          </div>
          <div className="text-sm text-gray-500 mb-3">
            A second encrypted copy of your backup in Ferry's cloud — for a lost or damaged USB.
          </div>
          <ul className="text-sm space-y-2">
            {[
              "Up to 50 GB per backup",
              "Kept 30 days, then deleted",
              "Sign in with your email — no password",
              "Encrypted on your PC before upload",
            ].map(f => (
              <li key={f} className="flex gap-2">
                <Check className="w-4 h-4 text-green-600 shrink-0 mt-0.5" /> {f}
              </li>
            ))}
          </ul>
        </div>
      </div>

      <div className="rounded-xl border border-gray-200 bg-white p-6 mb-6">
        <div className="font-semibold text-lg">Ferry for repair shops & IT</div>
        <p className="text-sm text-gray-600 mt-1">
          Per-seat subscription with a migration dashboard — for teams that reinstall machines
          regularly. Pricing to be set with early business customers.
        </p>
        <div className="mt-3 flex items-center gap-3 flex-wrap">
          <button
            onClick={() => {
              if (!account) {
                onAccount();
                return;
              }
              setPlannedTier(corporateChosen ? null : "Corporate");
            }}
            className={`text-sm px-4 py-1.5 rounded-lg border transition ${
              corporateChosen
                ? "border-brand-600 bg-brand-50 text-brand-700 font-medium"
                : "border-gray-300 hover:border-brand-600"
            }`}
          >
            {corporateChosen ? "✓ Corporate planned" : "Choose Corporate"}
          </button>
        </div>
        {corporateChosen && (
          <p className="text-xs text-gray-500 mt-2">
            Saved as your planned tier — shown again when business billing launches.
          </p>
        )}
      </div>

      <p className="text-xs text-gray-500 text-center">
        Two promises: your files are never monetized, and payment can never influence which
        download source the app picker recommends.
      </p>
    </div>
  );
}
