import type { CustomPresetConfig, OverlayConfig, PresetProfileConfig } from "./types";

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

export const CUSTOM_PRESET_LIMIT = 32;

export function addCustomPreset(config: OverlayConfig): OverlayConfig {
  if (config.presets.custom.length >= CUSTOM_PRESET_LIMIT) return config;
  const nextPreset: CustomPresetConfig = {
    name: `Custom ${config.presets.custom.length + 1}`,
    profile: currentProfile(config),
  };
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: [...config.presets.custom, nextPreset],
    },
  };
}

export function renameCustomPreset(config: OverlayConfig, index: number, name: string): OverlayConfig {
  return updateCustomPreset(config, index, (preset) => ({ ...preset, name }));
}

export function saveCurrentIntoCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  return updateCustomPreset(config, index, (preset) => ({ ...preset, profile: currentProfile(config) }));
}

export function deleteCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: config.presets.custom.filter((_, presetIndex) => presetIndex !== index),
    },
  };
}

function updateCustomPreset(
  config: OverlayConfig,
  index: number,
  update: (preset: CustomPresetConfig) => CustomPresetConfig,
): OverlayConfig {
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: config.presets.custom.map((preset, presetIndex) => (presetIndex === index ? update(preset) : preset)),
    },
  };
}

