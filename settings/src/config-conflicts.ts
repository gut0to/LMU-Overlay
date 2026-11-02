import { configurationChanges } from "./config-changes.ts";

export function overlappingPaths(left: string, right: string): boolean {
  return (
    left === right ||
    left.startsWith(`${right}.`) ||
    left.startsWith(`${right}[`) ||
    right.startsWith(`${left}.`) ||
    right.startsWith(`${left}[`)
  );
}

export type ConfigConflict = { path: string; editor: unknown; saved: unknown };
type PathNode = { children: Map<string, PathNode>; terminal: string | null };
const pathTokens = (path: string) => path.match(/[^.[\]]+/g) ?? [];
const pathNode = (): PathNode => ({ children: new Map(), terminal: null });

/** Index external changes once instead of comparing every pair of changed leaves. */
function indexPaths(paths: string[]) {
  const root = pathNode();
  for (const path of paths) {
    let node = root;
    for (const token of pathTokens(path)) {
      let child = node.children.get(token);
      if (!child) {
        child = pathNode();
        node.children.set(token, child);
      }
      node = child;
    }
    node.terminal = path;
  }
  return (path: string): string | null => {
    let node = root;
    for (const token of pathTokens(path)) {
      if (node.terminal !== null) return node.terminal;
      const child = node.children.get(token);
      if (!child) return null;
      node = child;
    }
    return node.terminal !== null || node.children.size > 0 ? path : null;
  };
}
export function compareConfiguration(
  baseline: unknown,
  editor: unknown,
  saved: unknown,
  limit = 50,
) {
  const local = configurationChanges(baseline, editor, Number.MAX_SAFE_INTEGER);
  const external = configurationChanges(
    baseline,
    saved,
    Number.MAX_SAFE_INTEGER,
  );
  const conflicts: ConfigConflict[] = [];
  const seen = new Set<string>();
  const commonPath = indexPaths(external.items.map((item) => item.path));
  for (const left of local.items) {
    const path = commonPath(left.path);
    if (path === null || seen.has(path)) continue;
    seen.add(path);
    const editorValue = valueAtPath(editor, path);
    const savedValue = valueAtPath(saved, path);
    if (configurationChanges(editorValue, savedValue, 0).total === 0) continue;
    conflicts.push({ path, editor: editorValue, saved: savedValue });
  }
  return {
    localCount: local.total,
    externalCount: external.total,
    conflictCount: conflicts.length,
    conflicts: conflicts.slice(0, limit),
    differences: configurationChanges(editor, saved, limit),
  };
}

function valueAtPath(value: unknown, path: string): unknown {
  for (const key of pathTokens(path)) {
    if (value === null || typeof value !== "object") return undefined;
    value = (value as Record<string, unknown>)[key];
  }
  return value;
}
