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
  return JSON.stringify(left) === JSON.stringify(right);
}
