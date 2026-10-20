import type { WindowConfig } from "./types";

const limits: Record<keyof WindowConfig, readonly [number, number, number]> = {
  x: [-2147483648, 2147483647, 0], y: [-2147483648, 2147483647, 0],
  width: [48, 7680, 640], height: [48, 7680, 480],
  refresh_hz: [15, 240, 60], sample_ms: [5, 1000, 10], history_samples: [16, 10000, 240],
};

export function normalizeWindowField(key: keyof WindowConfig, value: number): number {
  const [min, max, fallback] = limits[key];
  return Number.isFinite(value) ? Math.max(min, Math.min(max, Math.round(value))) : fallback;
}
