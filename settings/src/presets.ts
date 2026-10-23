import type { OverlayConfig, PresetProfileConfig } from "./types";

/** Capture an independent snapshot; later edits cannot change a saved preset. */
export function currentProfile(config: OverlayConfig): PresetProfileConfig {
  return structuredClone({
    performance_mode: config.performance.mode,
    reference_mode: config.timing.reference_mode,
    mini_sectors: config.timing.mini_sectors,
    style: config.style,
    units: config.units,
    coaching_config: config.coaching,
    layout: config.layout,
    extra_widgets: config.extra_widgets,
    ...config.widgets,
  });
}

export function profilesMatch(left: PresetProfileConfig, right: PresetProfileConfig): boolean {
  return canonicalJson(left) === canonicalJson(right);
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value).sort(([a], [b]) => a.localeCompare(b)).map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`).join(",")}}`;
  }
  return JSON.stringify(value);
}
