import { ShieldCheck, Usb, RotateCcw } from "lucide-react";

interface Props {
  onBackup: () => void;
  onRestore: () => void;
}

export function Welcome({ onBackup, onRestore }: Props) {
  return (
    <div className="max-w-2xl mx-auto text-center">
      <h2 className="text-2xl font-bold mb-3">Move to a new OS without losing your files</h2>
      <p className="text-gray-600 mb-8">
        Ferry backs up your personal files to a USB drive, downloads an official OS installer
        onto the same drive, and restores everything after the reinstall.
      </p>
      <div className="grid md:grid-cols-2 gap-4 text-left">
        <button
          onClick={onBackup}
          className="p-6 rounded-xl border border-gray-200 bg-white shadow-sm hover:border-brand-600 hover:shadow text-left transition"
        >
          <Usb className="w-8 h-8 text-brand-600 mb-3" />
          <div className="font-semibold text-lg">Back up & create installer</div>
          <p className="text-sm text-gray-500 mt-1">
            For this PC, before you wipe or switch systems.
          </p>
        </button>
        <button
          onClick={onRestore}
          className="p-6 rounded-xl border border-gray-200 bg-white shadow-sm hover:border-brand-600 hover:shadow text-left transition"
        >
          <RotateCcw className="w-8 h-8 text-brand-600 mb-3" />
          <div className="font-semibold text-lg">Restore from USB</div>
          <p className="text-sm text-gray-500 mt-1">
            For the new OS, after reinstalling. Needs your backup password.
          </p>
        </button>
      </div>
      <div className="mt-8 flex items-start gap-2 text-sm text-gray-500 text-left bg-white border border-gray-200 rounded-lg p-4">
        <ShieldCheck className="w-5 h-5 shrink-0 text-green-600" />
        <p>
          Ferry needs administrator access for USB and system operations. Your Windows digital
          license stays tied to this hardware and reactivates automatically after reinstalling
          the same edition.
        </p>
      </div>
    </div>
  );
}
