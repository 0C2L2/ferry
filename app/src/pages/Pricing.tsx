import { Check } from "lucide-react";
import { useAuth } from "../auth";

// Prices mirror company/business-model.md (illustrative, to be validated).
// Cloud Backup is post-MVP: shown honestly as "planned", never purchasable here.

const TIERS = [
  { size: "Up to 50 GB", price: "~$5" },
  { size: "Up to 200 GB", price: "~$15" },
  { size: "Up to 1 TB", price: "~$40" },
];

export function Pricing({ onAccount }: { onAccount: () => void }) {
  const { account } = useAuth();

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
          <div className="text-sm text-gray-500 mb-3">
            One-time charge per migration, priced by total backup size. A second encrypted copy
            in the cloud — for USBs that are too small, or just extra safety.
          </div>
          <ul className="text-sm space-y-2">
            {TIERS.map(t => (
              <li key={t.size} className="flex justify-between border-b border-gray-100 py-1.5">
                <span>{t.size}</span>
                <span className="font-medium">{t.price}</span>
              </li>
            ))}
          </ul>
          <button
            disabled
            title="Cloud Backup is not available in this build"
            className="mt-5 w-full px-4 py-2.5 rounded-lg bg-gray-200 text-gray-500 font-medium cursor-not-allowed"
          >
            Planned — not available in this build
          </button>
          {!account && (
            <button onClick={onAccount} className="mt-2 w-full text-sm text-brand-700 hover:text-brand-600">
              Create a free account to reserve your cloud identity →
            </button>
          )}
        </div>
      </div>

      <div className="rounded-xl border border-gray-200 bg-white p-6 mb-6">
        <div className="font-semibold text-lg">Ferry for repair shops & IT</div>
        <p className="text-sm text-gray-600 mt-1">
          Per-seat subscription with a migration dashboard — for teams that reinstall machines
          regularly. Pricing to be set with early business customers.
        </p>
        <div className="mt-3 inline-block text-sm px-3 py-1 rounded-full bg-gray-100 text-gray-600">
          Coming later
        </div>
      </div>

      <p className="text-xs text-gray-500 text-center">
        Two promises: your files are never monetized, and payment can never influence which
        download source the app picker recommends.
      </p>
    </div>
  );
}
