export type WorkspaceSize = { width: number; height: number; originX: number; originY: number };

/** Recover editor preferences without trusting persisted JSON types or dimensions. */
export function restoreWorkspace(saved: string | null, defaults: WorkspaceSize): WorkspaceSize {
  if (!saved) return { ...defaults };
  try {
    const parsed: unknown = JSON.parse(saved);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return { ...defaults };
    const value = parsed as Record<string, unknown>;
    const integer = (key: keyof WorkspaceSize) => {
      const candidate = value[key];
      return typeof candidate === "number" && Number.isFinite(candidate)
        ? Math.round(candidate)
        : defaults[key];
    };
    return {
      width: Math.max(320, Math.min(16384, integer("width"))),
      height: Math.max(240, Math.min(8640, integer("height"))),
      originX: Math.max(-2147483648, Math.min(2147483647, integer("originX"))),
      originY: Math.max(-2147483648, Math.min(2147483647, integer("originY"))),
    };
  } catch {
    return { ...defaults };
  }
}
