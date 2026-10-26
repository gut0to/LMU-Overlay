import type { WindowConfig } from "./types";

const limits: Record<keyof WindowConfig, readonly [number, number, number]> = {
  x: [-2147483648, 2147483647, 0], y: [-2147483648, 2147483647, 0],
  width: [280, 1200, 640], height: [140, 800, 480],
  refresh_hz: [15, 144, 60], sample_ms: [5, 250, 10], history_samples: [16, 900, 240],
};

export function normalizeWindowField(key: keyof WindowConfig, value: number): number {
  const [min, max, fallback] = limits[key];
  return Number.isFinite(value) ? Math.max(min, Math.min(max, Math.round(value))) : fallback;
}
