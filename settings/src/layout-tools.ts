import { normalizeWidgetLayoutForWindow, scaledDimension } from "./geometry.ts";
import type { WidgetLayout, WindowConfig } from "./types";

export type WidgetAlignment = "left" | "center-x" | "right" | "top" | "center-y" | "bottom";

/** Align visible bounds, including independent widget scales. */
export function alignWidgetToReference(window: WindowConfig, layout: WidgetLayout, reference: WidgetLayout, alignment: WidgetAlignment): WidgetLayout {
  if (layout.locked) return layout;
  const width = scaledDimension(layout.width, layout.scale);
  const height = scaledDimension(layout.height, layout.scale);
  const refWidth = scaledDimension(reference.width, reference.scale);
  const refHeight = scaledDimension(reference.height, reference.scale);
  let { x, y } = layout;
  switch (alignment) {
    case "left": x = reference.x; break;
    case "center-x": x = reference.x + (refWidth - width) / 2; break;
    case "right": x = reference.x + refWidth - width; break;
    case "top": y = reference.y; break;
    case "center-y": y = reference.y + (refHeight - height) / 2; break;
    case "bottom": y = reference.y + refHeight - height; break;
  }
  return normalizeWidgetLayoutForWindow(window, { ...layout, x, y });
}
