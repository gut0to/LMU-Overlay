export const DRAFT_STORAGE_KEY = "hashoverlay.settings.draft.v1";
export const MAX_DRAFT_LENGTH = 4 * 1024 * 1024;
export const DRAFT_RETENTION_MS = 30 * 24 * 60 * 60 * 1000;

export type ConfigDraft = {
  version: 1;
  path: string;
  base: string;
  savedAt: number;
  config: Record<string, unknown>;
};

export function createDraft(path: string, base: string, config: object, now = Date.now()): string {
  const serialized = JSON.stringify({ version: 1, path, base, savedAt: now, config });
  if (serialized.length > MAX_DRAFT_LENGTH) throw new Error("Local recovery draft exceeds the storage limit. Save or export your changes.");
  return serialized;
}
