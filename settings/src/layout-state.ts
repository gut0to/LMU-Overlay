import type { LayoutSelection, OverlayConfig, WidgetLayout } from "./types";

/** Per-surface edits must never rewrite the shared template used by its peers. */
export function updateOverlayLayoutSelection(config: OverlayConfig, overlayId: string, selection: LayoutSelection, layout: WidgetLayout): OverlayConfig {
  if (!config.overlays.some((overlay) => overlay.id === overlayId)) return config;
  const key = selection.startsWith("extra:") ? selection.slice("extra:".length) : selection;
  return {
    ...config,
    overlays: config.overlays.map((overlay) => overlay.id === overlayId
      ? { ...overlay, layout_overrides: { ...overlay.layout_overrides, [key]: { ...layout } } }
      : overlay),
  };
}
