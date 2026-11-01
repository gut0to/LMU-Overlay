export const SAVE_PREFERENCES_KEY = "hashoverlay.editor.save-preferences.v1";
export type SavePreferences = { version: 1; autosave: boolean };

export function restoreSavePreferences(raw: string | null): SavePreferences {
  const defaults: SavePreferences = { version: 1, autosave: true };
  if (!raw) return defaults;
  try {
    const value = JSON.parse(raw);
    return value && value.version === 1 && typeof value.autosave === "boolean" ? { version: 1, autosave: value.autosave } : defaults;
  } catch { return defaults; }
}
