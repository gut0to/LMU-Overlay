import { useCallback, useEffect, useRef, useState } from "react";
import { parseBackups, type ConfigBackup } from "./backups.ts";

export function useConfigBackups(owner: string, load: () => Promise<unknown>) {
  const [items, setItems] = useState<ConfigBackup[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  const currentOwner = useRef(owner);
  currentOwner.current = owner;
  const refresh = useCallback(async () => {
    if (!owner) return;
    const id = ++request.current;
    setLoading(true);
    setError(null);
    try {
      const result = parseBackups(await load());
      if (id === request.current && owner === currentOwner.current)
        setItems(result);
    } catch (error) {
      if (id === request.current && owner === currentOwner.current)
        setError(String(error));
    } finally {
      if (id === request.current && owner === currentOwner.current)
        setLoading(false);
    }
  }, [owner, load]);
  useEffect(() => {
    setItems([]);
    void refresh();
    return () => {
      request.current++;
    };
  }, [refresh]);
  return { items, loading, error, refresh };
}
