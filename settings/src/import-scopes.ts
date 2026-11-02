export const importScopes = [
  [
    "full",
    "Entire configuration",
    "Replace all settings, surfaces and saved presets.",
  ],
  [
    "appearance",
    "Appearance and units",
    "Copy the theme, units and per-widget styles. Keep layout and visibility.",
  ],
  [
    "timing",
    "Timing and coaching",
    "Copy reference timing and coaching preferences. Keep widget visibility.",
  ],
  ["hotkeys", "Hotkeys", "Copy global keyboard bindings only."],
  [
    "presets",
    "Saved presets",
    "Replace built-in and custom preset definitions. Keep the active setup.",
  ],
  [
    "surfaces",
    "Surface layout and visibility",
    "Copy surfaces, positions, sizes and widget membership. Keep styles, options and sampling rates.",
  ],
] as const;
export type ImportScope = (typeof importScopes)[number][0];
export function isImportScope(value: unknown): value is ImportScope {
  return importScopes.some(([scope]) => scope === value);
}
