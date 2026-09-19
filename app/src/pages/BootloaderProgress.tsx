import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { UsbLayout } from "../types";

interface Props {
  layout: UsbLayout;
  isoFilename: string;
  onDone: (warning: string | null) => void;
}

/** Final step: extracts the bootloader from the OS image already sitting on
 *  the data partition and writes a GRUB config that boots it. Not destructive
 *  — a failure here doesn't lose the backup or the OS image, so it's
 *  reported as a warning on the Done screen rather than a dead end. */
export function BootloaderProgress({ layout, isoFilename, onDone }: Props) {
  const [stage, setStage] = useState<StageState>("active");
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    (async () => {
      try {
        await api.writeBootloader(layout.boot_letter, layout.data_letter, isoFilename);
        setStage("done");
        onDone(null);
      } catch (err) {
        setStage("error");
        setError(String(err));
        onDone(String(err));
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Finishing USB</h2>
      <p className="text-sm text-gray-500 mb-4">
        Setting up the bootloader from the OS image so this USB can actually boot.
      </p>
      {error && (
        <ErrorBox
          message="Could not finish the bootloader. Your backup and the OS image are still safe on the USB."
          detail={error}
        />
      )}
      <StageList stages={[{ name: "Install bootloader from OS image", state: stage }]} />
    </div>
  );
}
