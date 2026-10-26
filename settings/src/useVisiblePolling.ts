import { useEffect, useRef } from "react";

/** Poll only after the previous request settles and while Settings is visible. */
export function useVisiblePolling(task: () => Promise<unknown>, interval = 1500) {
  const current = useRef(task);
  current.current = task;
  useEffect(() => {
    let active = true;
    let inFlight = false;
    let timer: ReturnType<typeof setTimeout> | null = null;
    const clear = () => { if (timer !== null) clearTimeout(timer); timer = null; };
    const tick = async () => {
      if (!active || inFlight || document.visibilityState === "hidden") return;
      clear();
      inFlight = true;
      try { await current.current(); }
      catch { /* The task owns its error presentation; keep future polling alive. */ }
      finally {
        inFlight = false;
        if (active && document.visibilityState !== "hidden") timer = setTimeout(() => void tick(), interval);
      }
    };
    const visibility = () => { clear(); if (document.visibilityState !== "hidden") void tick(); };
    document.addEventListener("visibilitychange", visibility);
    void tick();
    return () => { active = false; clear(); document.removeEventListener("visibilitychange", visibility); };
  }, [interval]);
}
