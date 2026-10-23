import type { OverlayConfig, OverlayLayerConfig, WidgetConfig, LayoutWidgetKey } from "./types";

export const legacyWidgetById: Partial<Record<string, keyof WidgetConfig>> = {
  telemetry: "speed_gear_rpm",
  inputs: "pedals",
  steering: "steering",
  lap_timing: "lap_timing",
  timing: "delta_timing",
  sectors: "sectors",
  mini_sectors: "mini_sector_widget",
  input_history: "input_history",
  coaching: "coaching",
  performance: "performance_monitor",
};

export const legacySurfaceByWidgetKey: Record<keyof WidgetConfig, LayoutWidgetKey | null> = {
  title: "telemetry",
  speed_gear_rpm: "telemetry",
  pedals: "inputs",
  steering: "inputs",
  lap_info: "lap_timing",
  lap_timing: "lap_timing",
  sectors: "sectors",
  mini_sector_widget: "mini_sectors",
  input_history: "inputs",
  delta_timing: "timing",
  ghost_inputs: "inputs",
  coaching: "coaching",
  performance_monitor: "performance",
};


export function activeLegacyWidgetEnabled(config: OverlayConfig, overlayId: string, key: keyof WidgetConfig): boolean {
  const surface = legacySurfaceByWidgetKey[key];
  if (!surface || !config.widgets[key]) {
    return config.widgets[key];
  }
  const overlay = config.overlays.find((item) => item.id === overlayId);
  return overlay ? overlay.widgets.includes(surface) : config.widgets[key];
}

export function catalogSurfaceId(id: string): string {
  const key = legacyWidgetById[id];
  return key ? legacySurfaceByWidgetKey[key] ?? id : id;
}

export function overlayIdUsedByOtherLayer(overlays: OverlayLayerConfig[], activeOverlayId: string, id: string): boolean {
  return overlays.some((overlay) => overlay.id !== activeOverlayId && overlay.widgets.includes(id));
}


export function catalogWidgetEnabled(config: OverlayConfig, overlayId: string, id: string): boolean {
  const legacy = legacyWidgetById[id];
  const globallyEnabled = legacy ? config.widgets[legacy] : config.extra_widgets[id]?.enabled ?? false;
  const overlay = config.overlays.find((item) => item.id === overlayId);
  return Boolean(globallyEnabled && (overlay ? overlay.widgets.includes(catalogSurfaceId(id)) : true));
}

/** Change membership once for an entire selection, keeping other surfaces alive. */
export function setCatalogWidgets(config: OverlayConfig, overlayId: string, ids: string[], enabled: boolean): OverlayConfig {
  if (!config.overlays.some((overlay) => overlay.id === overlayId)) return config;
  let next = config;
  for (const id of new Set(ids)) {
    const legacy = legacyWidgetById[id];
    if (!legacy && !config.extra_widgets[id]) continue;
    const surface = catalogSurfaceId(id);
    const overlays = next.overlays.map((overlay) => overlay.id === overlayId
      ? { ...overlay, widgets: enabled ? [...new Set([...overlay.widgets, surface])] : overlay.widgets.filter((widget) => widget !== surface) }
      : overlay);
    const needed = enabled || overlayIdUsedByOtherLayer(overlays, overlayId, surface);
    next = legacy
      ? { ...next, widgets: { ...next.widgets, [legacy]: needed }, overlays }
      : { ...next, extra_widgets: { ...next.extra_widgets, [id]: { ...next.extra_widgets[id], enabled: needed } }, overlays };
  }
  return next;
}
