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

export function addCustomPreset(config: OverlayConfig, profile?: PresetProfileConfig): OverlayConfig {
  if (config.presets.custom.length >= CUSTOM_PRESET_LIMIT) return config;
  const nextPreset: CustomPresetConfig = {
    name: uniquePresetName(config.presets.custom, "Custom"),
    profile: structuredClone(profile ?? currentProfile(config)),
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

export function saveCurrentIntoCustomPreset(config: OverlayConfig, index: number, profile = currentProfile(config)): OverlayConfig {
  return updateCustomPreset(config, index, (preset) => ({ ...preset, profile: structuredClone(profile) }));
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
  if (!config.presets.custom[index]) return config;
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: config.presets.custom.map((preset, presetIndex) => (presetIndex === index ? update(preset) : preset)),
    },
  };
}


/** Allocate a readable name even after deletions, imports or case-only renames. */
export function uniquePresetName(presets: Pick<CustomPresetConfig, "name">[], base: string): string {
  const names = new Set(presets.map((preset) => preset.name.trim().toLocaleLowerCase()));
  let suffix = 1;
  while (names.has(`${base} ${suffix}`.toLocaleLowerCase())) suffix++;
  return `${base} ${suffix}`;
}

export function duplicateCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  const source = config.presets.custom[index];
  if (!source || config.presets.custom.length >= CUSTOM_PRESET_LIMIT) return config;
  const copy = { name: uniquePresetName(config.presets.custom, `${source.name} copy`), profile: structuredClone(source.profile) };
  const custom = [...config.presets.custom];
  custom.splice(index + 1, 0, copy);
  return { ...config, presets: { ...config.presets, custom } };
}

export function moveCustomPreset(config: OverlayConfig, index: number, direction: -1 | 1): OverlayConfig {
  const custom = [...config.presets.custom];
  const target = index + direction;
  if (!custom[index] || target < 0 || target >= custom.length) return config;
  [custom[index], custom[target]] = [custom[target], custom[index]];
  return { ...config, presets: { ...config.presets, custom } };
}
