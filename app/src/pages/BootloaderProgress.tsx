import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useTauriProgress } from "../hooks/useTauriProgress";
import { ProgressBar, StageList, type StageState } from "../components/ProgressBar";
import { ErrorBox } from "../components/ErrorBox";
import type { ProgressPayload, UsbLayout } from "../types";

interface Props {
  layout: UsbLayout;
  isoFilename: string;
  onDone: (warning: string | null) => void;
}

/** Final step: extracts the downloaded OS image onto the bootable partition.
 *  Not destructive — a failure here doesn't lose the backup, so it's reported
 *  as a warning on the Done screen rather than a dead end. */
export function BootloaderProgress({ layout, isoFilename, onDone }: Props) {
  const [stage, setStage] = useState<StageState>("active");
  const [prog, setProg] = useState({ current: 0, total: 0 });
  const [item, setItem] = useState("Mounting the OS image…");
  const [error, setError] = useState<string | null>(null);
  const started = useRef(false);

  useTauriProgress("bootloader:progress", (p: ProgressPayload) => {
    setProg({ current: p.current, total: p.total });
    setItem(p.current_item);
  });

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

  const pct = prog.total > 0 ? Math.round((prog.current / prog.total) * 100) : 0;

  return (
    <div className="max-w-xl mx-auto">
      <h2 className="text-xl font-semibold mb-4">Finishing USB</h2>
      <p className="text-sm text-gray-500 mb-4">
        Unpacking the OS installer onto the boot partition. This moves several gigabytes and
        takes a few minutes.
      </p>
      {error && (
        <ErrorBox
          message="Could not finish the bootable USB. Your backup is still safe on the drive."
          detail={error}
        />
      )}
      {!error && (
        <ProgressBar
          current={prog.current}
          total={prog.total}
          label={prog.total > 0 ? `${item} · ${pct}%` : item}
        />
      )}
      <StageList stages={[{ name: "Unpack OS installer to boot partition", state: stage }]} />
      <p className="text-xs text-gray-500 mt-4">Do not remove the USB drive until this finishes.</p>
    </div>
  );
}
