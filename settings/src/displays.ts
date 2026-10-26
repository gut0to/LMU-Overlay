import { restoreWorkspace, type WorkspaceSize } from "./workspace.ts";

export type DisplayRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};
export type Display = {
  id: string;
  name: string;
  primary: boolean;
  bounds: DisplayRect;
  work_area: DisplayRect;
};

export function parseDisplays(value: unknown): Display[] {
  if (!Array.isArray(value)) return [];
  const validRect = (rect: unknown): rect is DisplayRect => {
    if (!rect || typeof rect !== "object") return false;
    const value = rect as DisplayRect;
    return (
      [value.x, value.y, value.width, value.height].every(
        Number.isSafeInteger,
      ) &&
      value.width > 0 &&
      value.height > 0
    );
  };
  const ids = new Set<string>();
  return value.filter((display): display is Display => {
    if (
      !display ||
      typeof display.id !== "string" ||
      !display.id ||
      ids.has(display.id) ||
      typeof display.name !== "string" ||
      typeof display.primary !== "boolean" ||
      !validRect(display.bounds) ||
      !validRect(display.work_area)
    )
      return false;
    ids.add(display.id);
    return true;
  });
}

export function displayWorkspace(
  display: Display,
  workArea: boolean,
): WorkspaceSize {
  const rect = workArea ? display.work_area : display.bounds;
  return restoreWorkspace(
    JSON.stringify({
      width: rect.width,
      height: rect.height,
      originX: rect.x,
      originY: rect.y,
    }),
    { width: 1920, height: 1080, originX: 0, originY: 0 },
  );
}

export function matchesDisplay(
  workspace: WorkspaceSize,
  display: Display,
  workArea: boolean,
): boolean {
  const target = displayWorkspace(display, workArea);
  return (
    workspace.width === target.width &&
    workspace.height === target.height &&
    workspace.originX === target.originX &&
    workspace.originY === target.originY
  );
}
