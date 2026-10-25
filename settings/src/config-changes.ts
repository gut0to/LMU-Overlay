export type ConfigChange = { path: string; before: unknown; after: unknown };
export type ConfigChanges = { total: number; items: ConfigChange[] };

/** Count every changed leaf while bounding the amount rendered in the review. */
export function configurationChanges(
  before: unknown,
  after: unknown,
  limit = 80,
): ConfigChanges {
  const result: ConfigChanges = { total: 0, items: [] };
  const record = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value);
  const visit = (left: unknown, right: unknown, path: string) => {
    if (left === right) return;
    if (record(left) && record(right)) {
      for (const key of [
        ...new Set([...Object.keys(left), ...Object.keys(right)]),
      ].sort()) {
        visit(left[key], right[key], path ? `${path}.${key}` : key);
      }
      return;
    }
    if (Array.isArray(left) && Array.isArray(right)) {
      for (let index = 0; index < Math.max(left.length, right.length); index++)
        visit(left[index], right[index], `${path}[${index}]`);
      return;
    }
    result.total++;
    if (result.items.length < Math.max(0, limit))
      result.items.push({ path, before: left, after: right });
  };
  visit(before, after, "");
  return result;
}

export function formatChangeValue(value: unknown): string {
  if (value === undefined) return "Not set";
  if (value === "") return '""';
  const text = typeof value === "string" ? value : JSON.stringify(value);
  return text.length > 160 ? `${text.slice(0, 157)}…` : text;
}
