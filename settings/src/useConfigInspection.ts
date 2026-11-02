import { useCallback, useEffect, useRef, useState } from "react";
import type { OverlayConfig } from "./types";

export type ConfigInspection = { path: string; revision: string; config: OverlayConfig; text: string };
export function useConfigInspection(editorKey: string, load: () => Promise<ConfigInspection>) {
  const [state, setState] = useState<{ key: string; result: ConfigInspection | null; pending: boolean; error: string | null } | null>(null);
  const request = useRef(0);
  const current = useRef(editorKey);
  current.current = editorKey;
  useEffect(() => () => { request.current++; }, []);
  const clear = useCallback(() => { request.current++; setState(null); }, []);
  const inspect = useCallback(async () => {
    const id = ++request.current;
    setState({ key: editorKey, result: null, pending: true, error: null });
    try {
      const result = await load();
      if (id === request.current && current.current === editorKey) setState({ key: editorKey, result, pending: false, error: null });
    } catch (error) {
      if (id === request.current && current.current === editorKey) setState({ key: editorKey, result: null, pending: false, error: String(error) });
    }
  }, [editorKey, load]);
  const active = state?.key === editorKey ? state : null;
  return { result: active?.result ?? null, pending: active?.pending ?? false, error: active?.error ?? null, inspect, clear };
}
