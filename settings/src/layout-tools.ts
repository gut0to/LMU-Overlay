import { normalizeWidgetLayoutForWindow, scaledDimension } from "./geometry.ts";
import type { WidgetLayout, WindowConfig } from "./types";

export type WidgetAlignment =
  | "left"
  | "center-x"
  | "right"
  | "top"
  | "center-y"
  | "bottom";

/** Align visible bounds, including independent widget scales. */
export function alignWidgetToReference(
  window: WindowConfig,
  layout: WidgetLayout,
  reference: WidgetLayout,
  alignment: WidgetAlignment,
): WidgetLayout {
  if (layout.locked) return layout;
  const width = scaledDimension(layout.width, layout.scale);
  const height = scaledDimension(layout.height, layout.scale);
  const refWidth = scaledDimension(reference.width, reference.scale);
  const refHeight = scaledDimension(reference.height, reference.scale);
  let { x, y } = layout;
  switch (alignment) {
    case "left":
      x = reference.x;
      break;
    case "center-x":
      x = reference.x + (refWidth - width) / 2;
      break;
    case "right":
      x = reference.x + refWidth - width;
      break;
    case "top":
      y = reference.y;
      break;
    case "center-y":
      y = reference.y + (refHeight - height) / 2;
      break;
    case "bottom":
      y = reference.y + refHeight - height;
      break;
  }
  return normalizeWidgetLayoutForWindow(window, { ...layout, x, y });
}

export function nudgeWidget(
  window: WindowConfig,
  layout: WidgetLayout,
  dx: number,
  dy: number,
): WidgetLayout {
  if (layout.locked || !Number.isFinite(dx) || !Number.isFinite(dy))
    return layout;
  return normalizeWidgetLayoutForWindow(window, {
    ...layout,
    x: layout.x + dx,
    y: layout.y + dy,
  });
}

export function moveWidgetInStack(
  layout: WidgetLayout,
  neighbors: WidgetLayout[],
  direction: "front" | "back",
): WidgetLayout {
  if (layout.locked) return layout;
  const orders = neighbors
    .map((neighbor) => neighbor.z_index)
    .filter(Number.isFinite);
  const z_index =
    direction === "front"
      ? Math.min(1000, Math.max(layout.z_index, ...orders) + 1)
      : Math.max(-1000, Math.min(layout.z_index, ...orders) - 1);
  return { ...layout, z_index };
}

export function fitWindowToWidgets(
  window: WindowConfig,
  layouts: WidgetLayout[],
  padding = 14,
): WindowConfig {
  if (layouts.length === 0) return window;
  const gap = Number.isFinite(padding) ? Math.max(0, padding) : 14;
  return {
    ...window,
    width: Math.min(
      1200,
      Math.max(
        280,
        Math.ceil(
          Math.max(
            ...layouts.map(
              (layout) =>
                layout.x + scaledDimension(layout.width, layout.scale),
            ),
          ) + gap,
        ),
      ),
    ),
    height: Math.min(
      800,
      Math.max(
        140,
        Math.ceil(
          Math.max(
            ...layouts.map(
              (layout) =>
                layout.y + scaledDimension(layout.height, layout.scale),
            ),
          ) + gap,
        ),
      ),
    ),
  };
}
