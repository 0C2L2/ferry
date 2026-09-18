import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ProgressPayload } from "../types";

/** Subscribe to a Tauri progress event (e.g. "backup:progress"). */
export function useTauriProgress(
  event: string,
  onProgress: (p: ProgressPayload) => void,
) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ProgressPayload>(event, e => onProgress(e.payload)).then(u => {
      unlisten = u;
    });
    return () => {
      unlisten?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [event]);
}
