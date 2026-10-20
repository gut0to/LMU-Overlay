import type { WidgetLayout, WindowConfig } from "./types";

function clamp(value: number, min: number, max: number) { return Math.min(Math.max(value, min), max); }

export function normalizeWidgetLayoutForWindow(window: WindowConfig, layout: WidgetLayout): WidgetLayout {
  const scale = clamp(Number.isFinite(layout.scale) ? layout.scale : 1, 0.5, 2);
  const width = clamp(
    Number.isFinite(layout.width) ? Math.round(layout.width) : 48,
    48,
    Math.max(48, Math.floor(window.width / scale)),
  );
  const height = clamp(
    Number.isFinite(layout.height) ? Math.round(layout.height) : 20,
    20,
    Math.max(20, Math.floor(window.height / scale)),
  );
  const visualWidth = scaledDimension(width, scale);
  const visualHeight = scaledDimension(height, scale);
  return {
    ...layout,
    x: clamp(Number.isFinite(layout.x) ? Math.round(layout.x) : 0, 0, Math.max(0, window.width - visualWidth)),
    y: clamp(Number.isFinite(layout.y) ? Math.round(layout.y) : 0, 0, Math.max(0, window.height - visualHeight)),
    width,
    height,
    scale,
    opacity: clamp(Number.isFinite(layout.opacity) ? layout.opacity : 1, 0.1, 1),
    z_index: Number.isFinite(layout.z_index) ? Math.round(layout.z_index) : 0,
  };
}

export function scaledDimension(value: number, scale: number) {
  return Math.round(value * clamp(scale, 0.5, 2));
}

