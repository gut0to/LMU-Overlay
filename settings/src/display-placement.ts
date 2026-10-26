import type { WindowConfig } from "./types";
import type { WorkspaceSize } from "./workspace";
import { normalizeWindowField } from "./window-values.ts";

export function fitWindowToWorkspace(window: WindowConfig, workspace: WorkspaceSize): WindowConfig {
  const width = Math.min(workspace.width, normalizeWindowField("width", window.width));
  const height = Math.min(workspace.height, normalizeWindowField("height", window.height));
  return {
    ...window, width, height,
    x: Math.max(workspace.originX, Math.min(window.x, workspace.originX + workspace.width - width)),
    y: Math.max(workspace.originY, Math.min(window.y, workspace.originY + workspace.height - height)),
  };
}
