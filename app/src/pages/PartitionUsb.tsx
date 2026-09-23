import { useState } from "react";
import { TriangleAlert } from "lucide-react";
import { api } from "../api";
import { StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import { formatBytes, type DriveInfo, type OsSource, type UsbLayout } from "../types";

interface Props {
  drive: DriveInfo;
  os: OsSource;
  onDone: (layout: UsbLayout) => void;
}

/** The boot partition holds the OS image's extracted contents, so it is sized
 *  to the image plus headroom for FAT32 overhead and cluster slack. Falls back
 *  to 8 GB when a source ships no size estimate. */
function bootPartitionMb(os: OsSource): number {
  if (!os.approx_bytes) return 8192;
  return Math.ceil((os.approx_bytes * 1.15) / (1024 * 1024));
}

/** The actual point of no return: erases the drive and lays down the
 *  FAT32 boot + exFAT data partitions. Runs immediately after drive
 *  selection, before backup ever writes anything — so nothing downstream
 *  can be destroyed by this step, because nothing downstream exists yet. */
export function PartitionUsb({ drive, os, onDone }: Props) {
  const [confirmed, setConfirmed] = useState(false);
  const [running, setRunning] = useState(false);
  const [stage, setStage] = useState<StageState>("pending");
  const [error, setError] = useState<string | null>(null);

  async function run() {
    setRunning(true);
    setError(null);
    setStage("active");
    try {
      const layout = await api.prepareUsb(drive.drive_letter, bootPartitionMb(os));
      setStage("done");
      onDone(layout);
    } catch (err) {
      setStage("error");
      setError(String(err));
      setRunning(false);
    }
  }

  const bootMb = bootPartitionMb(os);
  // Catch an undersized stick here rather than letting diskpart fail with
  // something the user can't act on.
  const tooSmall = drive.total_bytes > 0 && drive.total_bytes < bootMb * 1024 * 1024 * 1.1;

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Prepare USB drive</h2>
      {tooSmall && (
        <ErrorBox
          message={`This drive is too small for ${os.label}.`}
          detail={`${os.label} needs about ${formatBytes(bootMb * 1024 * 1024)} for the installer alone, plus room for your backup. ${drive.model} holds ${formatBytes(drive.total_bytes)}. Go back and choose a larger drive.`}
          expanded
        />
      )}
      {!running && !error && !tooSmall && (
        <>
          <div className="bg-red-50 border border-red-200 text-red-900 rounded-lg p-4 mb-4 flex gap-2 text-sm">
            <TriangleAlert className="w-5 h-5 shrink-0" />
            <p>
              This will <strong>erase everything currently on</strong> {drive.drive_letter} (
              {drive.model}) and set it up as a Ferry USB — a{" "}
              {formatBytes(bootMb * 1024 * 1024)} boot partition for the {os.label} installer,
              plus the rest as storage for your backup. Nothing on your PC itself is touched,
              only this drive.
            </p>
          </div>
          <label className="flex items-start gap-2 text-sm mb-6">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={e => setConfirmed(e.target.checked)}
              className="mt-1"
            />
            I understand this will erase this drive permanently.
          </label>
          <button
            onClick={run}
            disabled={!confirmed}
            className="px-6 py-2.5 rounded-lg bg-red-600 text-white font-medium disabled:opacity-40 hover:bg-red-700"
          >
            Erase & prepare drive
          </button>
        </>
      )}
      {error && <ErrorBox message="Could not prepare the USB drive." detail={error} />}
      {(running || error) && (
        <StageList stages={[{ name: "Partition & format (FAT32 boot + exFAT data)", state: stage }]} />
      )}
    </div>
  );
}
