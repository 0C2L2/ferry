import { CheckCircle2, TriangleAlert } from "lucide-react";
import type { Manifest, OsSource } from "../types";

interface Props {
  dataRoot: string;
  os: OsSource;
  manifest: Manifest | null;
  bootloaderWarning: string | null;
  onRestart: () => void;
}

export function Done({ dataRoot, os, manifest, bootloaderWarning, onRestart }: Props) {
  return (
    <div className="max-w-xl mx-auto text-center">
      <CheckCircle2 className="w-12 h-12 text-green-600 mx-auto mb-3" />
      <h2 className="text-2xl font-bold mb-2">USB ready</h2>
      <p className="text-gray-600 mb-6">
        {dataRoot} now holds the {os.label} installer
        {manifest ? ` and your encrypted backup (${manifest.files.length} files)` : ""}.
      </p>
      {bootloaderWarning && (
        <div className="bg-amber-50 border border-amber-200 text-amber-900 rounded-lg p-4 mb-4 flex gap-2 text-sm text-left">
          <TriangleAlert className="w-5 h-5 shrink-0" />
          <p>Bootloader note: {bootloaderWarning}</p>
        </div>
      )}
      <ol className="text-left text-sm bg-white border border-gray-200 rounded-xl p-5 space-y-2 mb-6">
        <li>1. Shut down this PC and plug in the USB drive.</li>
        <li>2. Boot from USB (usually F12, F2, Esc or Del at startup).</li>
        <li>3. Install {os.label}.</li>
        <li>4. On the new system, run Ferry from the USB and choose “Restore from USB”.</li>
      </ol>
      <button onClick={onRestart} className="text-brand-700 hover:text-brand-600 text-sm">
        ← Back to start
      </button>
    </div>
  );
}
