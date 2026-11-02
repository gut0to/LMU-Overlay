import { configurationChanges, type ConfigChange } from "./config-changes.ts";

export function overlappingPaths(left: string, right: string): boolean {
  return left === right || left.startsWith(`${right}.`) || left.startsWith(`${right}[`) || right.startsWith(`${left}.`) || right.startsWith(`${left}[`);
}

export type ConfigConflict = { path: string; editor: unknown; saved: unknown };
export function compareConfiguration(baseline: unknown, editor: unknown, saved: unknown, limit = 50) {
  const local = configurationChanges(baseline, editor, Number.MAX_SAFE_INTEGER);
  const external = configurationChanges(baseline, saved, Number.MAX_SAFE_INTEGER);
  const conflicts: ConfigConflict[] = [];
  const seen = new Set<string>();
  for (const left of local.items) {
    for (const right of external.items) {
      if (!overlappingPaths(left.path, right.path) || JSON.stringify(left.after) === JSON.stringify(right.after)) continue;
      const path = left.path.length <= right.path.length ? left.path : right.path;
      if (!seen.has(path)) { seen.add(path); conflicts.push({ path, editor: left.after, saved: right.after }); }
    }
  }
  return { localCount: local.total, externalCount: external.total, conflictCount: conflicts.length,
    conflicts: conflicts.slice(0, limit), differences: configurationChanges(editor, saved, limit) };
}
