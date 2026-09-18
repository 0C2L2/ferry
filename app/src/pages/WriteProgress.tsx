import { useState } from "react";
import { TriangleAlert } from "lucide-react";
import { api } from "../api";
import { StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { DriveInfo } from "../types";

interface Props {
  drive: DriveInfo;
  onDone: (bootloaderWarning: string | null) => void;
}

export function WriteProgress({ drive, onDone }: Props) {
  const [confirmed, setConfirmed] = useState(false);
  const [running, setRunning] = useState(false);
  const [stages, setStages] = useState<StageState[]>(["pending", "pending"]);
  const [error, setError] = useState<string | null>(null);

  async function run() {
    setRunning(true);
    setError(null);
    let warning: string | null = null;
    try {
      setStages(["active", "pending"]);
      await api.prepareUsb(drive.drive_letter, drive.drive_letter);
      setStages(["done", "active"]);
      try {
        await api.writeBootloader(drive.drive_letter);
      } catch (err) {
        warning = String(err);
      }
      setStages(["done", warning ? "error" : "done"]);
      onDone(warning);
    } catch (err) {
      setStages(prev => prev.map(v => (v === "active" ? "error" : v)));
      setError(String(err));
      setRunning(false);
    }
  }

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Write bootable USB</h2>
      {!running && !error && (
        <>
          <div className="bg-red-50 border border-red-200 text-red-900 rounded-lg p-4 mb-4 flex gap-2 text-sm">
            <TriangleAlert className="w-5 h-5 shrink-0" />
            <p>
              This will <strong>erase everything</strong> on {drive.drive_letter} ({drive.model})
              and repartition it. Your files are already backed up and encrypted on this same
              drive — that backup was checksum-verified before this step unlocked.
            </p>
          </div>
          <label className="flex items-start gap-2 text-sm mb-6">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={e => setConfirmed(e.target.checked)}
              className="mt-1"
            />
            I understand this will erase the drive permanently.
          </label>
          <button
            onClick={run}
            disabled={!confirmed}
            className="px-6 py-2.5 rounded-lg bg-red-600 text-white font-medium disabled:opacity-40 hover:bg-red-700"
          >
            Erase & write USB
          </button>
        </>
      )}
      {error && <ErrorBox message="USB preparation failed." detail={error} />}
      {(running || error) && (
        <StageList
          stages={[
            { name: "Partition & format (FAT32 boot + exFAT data)", state: stages[0] },
            { name: "Write bootloader", state: stages[1] },
          ]}
        />
      )}
    </div>
  );
}
