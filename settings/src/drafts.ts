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

export function parseDraft(raw: string | null, now = Date.now()): ConfigDraft | null {
  if (!raw || raw.length > MAX_DRAFT_LENGTH) return null;
  try {
    const draft = JSON.parse(raw);
    if (!draft || typeof draft !== "object" || Array.isArray(draft)
      || draft.version !== 1 || typeof draft.path !== "string" || !draft.path.trim()
      || typeof draft.base !== "string" || !draft.base
      || !Number.isSafeInteger(draft.savedAt) || draft.savedAt < now - DRAFT_RETENTION_MS
      || draft.savedAt > now + 5 * 60 * 1000
      || !draft.config || typeof draft.config !== "object" || Array.isArray(draft.config)) return null;
    return draft;
  } catch { return null; }
}
