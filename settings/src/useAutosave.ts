import { useCallback, useEffect, useRef } from "react";

/** Save only settled edits; active gestures, manual mode and file replacement own their state. */
export function useAutosave<T>(
  value: T | null,
  saved: string,
  failed: string | null,
  enabled: boolean,
  save: (value: T) => Promise<unknown>,
  delay = 850,
) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const currentSave = useRef(save);
  currentSave.current = save;
  const cancel = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  }, []);
  useEffect(() => {
    cancel();
    if (!enabled || value === null) return;
    const serialized = JSON.stringify(value);
    if (serialized === saved || serialized === failed) return;
    timer.current = setTimeout(() => {
      timer.current = null;
      void currentSave.current(value);
    }, delay);
    return cancel;
  }, [value, saved, failed, enabled, delay, cancel]);
  return cancel;
}
