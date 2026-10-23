import type {
  OverlayConfig,
  OverlayLayerConfig,
  WidgetConfig,
  LayoutWidgetKey,
} from "./types";

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

export const legacySurfaceByWidgetKey: Record<
  keyof WidgetConfig,
  LayoutWidgetKey | null
> = {
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

export function activeLegacyWidgetEnabled(
  config: OverlayConfig,
  overlayId: string,
  key: keyof WidgetConfig,
): boolean {
  const surface = legacySurfaceByWidgetKey[key];
  if (!surface || !config.widgets[key]) {
    return config.widgets[key];
  }
  const overlay = config.overlays.find((item) => item.id === overlayId);
  return overlay ? overlay.widgets.includes(surface) : config.widgets[key];
}

export function catalogSurfaceId(id: string): string {
  const key = legacyWidgetById[id];
  return key ? (legacySurfaceByWidgetKey[key] ?? id) : id;
}

export function overlayIdUsedByOtherLayer(
  overlays: OverlayLayerConfig[],
  activeOverlayId: string,
  id: string,
): boolean {
  return overlays.some(
    (overlay) => overlay.id !== activeOverlayId && overlay.widgets.includes(id),
  );
}

export function catalogWidgetEnabled(
  config: OverlayConfig,
  overlayId: string,
  id: string,
): boolean {
  const legacy = legacyWidgetById[id];
  const globallyEnabled = legacy
    ? config.widgets[legacy]
    : (config.extra_widgets[id]?.enabled ?? false);
  const overlay = config.overlays.find((item) => item.id === overlayId);
  return Boolean(
    globallyEnabled &&
    (overlay ? overlay.widgets.includes(catalogSurfaceId(id)) : true),
  );
}

/** Change membership once for an entire selection, keeping other surfaces alive. */
export function setCatalogWidgets(
  config: OverlayConfig,
  overlayId: string,
  ids: string[],
  enabled: boolean,
): OverlayConfig {
  if (!config.overlays.some((overlay) => overlay.id === overlayId))
    return config;
  let next = config;
  for (const id of new Set(ids)) {
    const legacy = legacyWidgetById[id];
    if (!legacy && !config.extra_widgets[id]) continue;
    const surface = catalogSurfaceId(id);
    const overlays = next.overlays.map((overlay) =>
      overlay.id === overlayId
        ? {
            ...overlay,
            widgets: enabled
              ? [...new Set([...overlay.widgets, surface])]
              : overlay.widgets.filter((widget) => widget !== surface),
          }
        : overlay,
    );
    const needed =
      enabled || overlayIdUsedByOtherLayer(overlays, overlayId, surface);
    next = legacy
      ? { ...next, widgets: { ...next.widgets, [legacy]: needed }, overlays }
      : {
          ...next,
          extra_widgets: {
            ...next.extra_widgets,
            [id]: { ...next.extra_widgets[id], enabled: needed },
          },
          overlays,
        };
  }
  return next;
}

/** Applying a surface preset must not turn off widgets assigned elsewhere. */
export function preserveOtherSurfaceWidgets(
  previous: OverlayConfig,
  next: OverlayConfig,
  overlayId: string,
): OverlayConfig {
  const widgets = { ...next.widgets };
  for (const [key, surface] of Object.entries(legacySurfaceByWidgetKey)) {
    if (
      surface &&
      overlayIdUsedByOtherLayer(next.overlays, overlayId, surface)
    ) {
      const widget = key as keyof WidgetConfig;
      widgets[widget] ||= previous.widgets[widget];
    }
  }
  const extra_widgets = { ...next.extra_widgets };
  for (const [id, widget] of Object.entries(previous.extra_widgets)) {
    if (
      widget.enabled &&
      overlayIdUsedByOtherLayer(next.overlays, overlayId, id)
    ) {
      extra_widgets[id] = { ...(extra_widgets[id] ?? widget), enabled: true };
    }
  }
  return { ...next, widgets, extra_widgets };
}

export function synchronizeWidgetMembership(
  config: OverlayConfig,
): OverlayConfig {
  const used = new Set(config.overlays.flatMap((overlay) => overlay.widgets));
  const widgets = { ...config.widgets };
  for (const id of used) {
    const key = legacyWidgetById[id];
    if (
      key &&
      !Object.entries(legacySurfaceByWidgetKey).some(
        ([member, surface]) =>
          surface === id && widgets[member as keyof WidgetConfig],
      )
    ) {
      widgets[key] = true;
    }
  }
  return {
    ...config,
    widgets,
    extra_widgets: Object.fromEntries(
      Object.entries(config.extra_widgets).map(([id, widget]) => [
        id,
        used.has(id) ? { ...widget, enabled: true } : widget,
      ]),
    ),
  };
}
