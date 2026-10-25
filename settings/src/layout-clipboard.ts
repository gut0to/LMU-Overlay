import { normalizeWidgetLayoutForWindow, scaledDimension } from "./geometry.ts";
import type { WidgetLayout, WindowConfig } from "./types";

export type LayoutClipboard = { source: string; geometry: Pick<WidgetLayout, "x" | "y" | "width" | "height" | "scale"> };
export type LayoutPaste = "position" | "size" | "geometry";

export function copyWidgetLayout(source: string, layout: WidgetLayout): LayoutClipboard {
  const { x, y, width, height, scale } = layout;
  return { source, geometry: { x, y, width, height, scale } };
}

/** Paste only geometry: target opacity, lock and stacking order remain its own. */
export function pasteWidgetLayout(window: WindowConfig, target: WidgetLayout, clipboard: LayoutClipboard, mode: LayoutPaste): WidgetLayout {
  if (target.locked) return target;
  const copied = clipboard.geometry;
  const geometry = mode === "position" ? { x: copied.x, y: copied.y }
    : mode === "size" ? { width: Math.round(scaledDimension(copied.width, copied.scale) / target.scale), height: Math.round(scaledDimension(copied.height, copied.scale) / target.scale) }
    : copied;
  return normalizeWidgetLayoutForWindow(window, { ...target, ...geometry });
}
