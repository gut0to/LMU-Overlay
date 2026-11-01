export type ConfigBackup = {
  id: string;
  modified_ms: number;
  size_bytes: number;
};

export function parseBackups(raw: unknown): ConfigBackup[] {
  if (!Array.isArray(raw)) return [];
  const seen = new Set<string>();
  return raw
    .filter((value): value is ConfigBackup => {
      if (
        !value ||
        typeof value.id !== "string" ||
        !/^[0-9][0-9-]{0,95}\.toml$/.test(value.id) ||
        seen.has(value.id) ||
        !Number.isSafeInteger(value.modified_ms) ||
        value.modified_ms < 0 ||
        !Number.isSafeInteger(value.size_bytes) ||
        value.size_bytes < 0
      )
        return false;
      seen.add(value.id);
      return true;
    })
    .sort((left, right) => right.id.localeCompare(left.id));
}

export function backupTime(milliseconds: number): string {
  const date = new Date(milliseconds);
  return milliseconds > 0 && Number.isFinite(date.getTime())
    ? date.toLocaleString()
    : "Unknown time";
}
