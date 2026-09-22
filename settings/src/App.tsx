import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Copy,
  FolderOpen,
  Gauge,
  LayoutGrid,
  Lock,
  Magnet,
  Paintbrush,
  Play,
  Plus,
  RotateCcw,
  Save,
  SlidersHorizontal,
  Square,
  Timer,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CoachingConfig,
  CustomPresetConfig,
  HotkeyConfig,
  LayoutConfig,
  LayoutSelection,
  LayoutWidgetKey,
  LoadResponse,
  OverlayConfig,
  OverlayLayerConfig,
  PerformanceConfig,
  PresetConfig,
  PresetProfileConfig,
  StyleConfig,
  TimingConfig,
  UnitsConfig,
  WidgetConfig,
  WidgetDefinition,
  WidgetInstanceConfig,
  WidgetLayout,
  WidgetOptions,
  WidgetStyleConfig,
  WindowConfig,
} from "./types";

type SectionProps = {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
};

const presetNames = ["practice", "qualifying", "race", "endurance", "minimal"] as const;
type StandardPresetKey = (typeof presetNames)[number];
const pages = ["Dashboard", "Widgets", "Layout", "Appearance", "Timing", "Coaching", "Performance", "Hotkeys", "Presets", "Advanced"] as const;
type Page = (typeof pages)[number];
type WorkspaceSize = { width: number; height: number; originX: number; originY: number };
const workspacePresets: Array<[string, WorkspaceSize]> = [
  ["Primary 1080p", { width: 1920, height: 1080, originX: 0, originY: 0 }],
  ["Primary 1440p", { width: 2560, height: 1440, originX: 0, originY: 0 }],
  ["Left monitor", { width: 1920, height: 1080, originX: -1920, originY: 0 }],
  ["Right monitor", { width: 1920, height: 1080, originX: 1920, originY: 0 }],
  ["Ultrawide", { width: 3440, height: 1440, originX: 0, originY: 0 }],
  ["4K", { width: 3840, height: 2160, originX: 0, originY: 0 }],
];
const workspaceStorageKey = "hashoverlay.editor.workspace";
const performanceModes = [
  ["eco", "Eco"],
  ["normal", "Normal"],
  ["high_refresh", "High Refresh"],
  ["custom", "Custom"],
];
const performanceWindowValues: Record<string, Pick<WindowConfig, "refresh_hz" | "sample_ms"> | undefined> = {
  eco: { refresh_hz: 30, sample_ms: 20 },
  normal: { refresh_hz: 60, sample_ms: 10 },
  high_refresh: { refresh_hz: 120, sample_ms: 10 },
  custom: undefined,
};
const referenceModes = [
  ["personal_best", "Personal Best"],
  ["session_best", "Session Best"],
  ["best_valid_lap", "Best Valid Lap"],
  ["last_lap", "Last Complete Lap"],
];
const speedUnits = [
  ["kmh", "km/h"],
  ["mph", "mph"],
];
const themes = [
  ["hashoverlay_default", "HashOverlay Default"],
  ["minimal_dark", "Minimal Dark"],
  ["transparent", "Transparent"],
  ["high_contrast", "High Contrast"],
  ["custom", "Custom"],
];
const coachingModes = [
  ["off", "Off"],
  ["race", "Race"],
  ["practice", "Practice"],
  ["attack", "Attack"],
];
const temperatureUnits = [
  ["celsius", "Celsius"],
  ["fahrenheit", "Fahrenheit"],
];
const pressureUnits = [
  ["kpa", "kPa"],
  ["psi", "psi"],
];
const tyreTemperatureModes = [
  ["surface_average", "Surface avg"],
  ["surface_lcr", "Surface L/C/R"],
  ["carcass", "Carcass"],
  ["inner_layer", "Inner layer"],
];
const fuelUnits = [
  ["liters", "Liters"],
  ["gallons", "Gallons"],
];
const gridSizes = [
  ["5", "5 px"],
  ["10", "10 px"],
  ["20", "20 px"],
];
const coachingLabels: Array<[keyof Pick<CoachingConfig, "brake_timing" | "throttle_timing" | "input_match" | "speed" | "gear">, string]> = [
  ["brake_timing", "Brake timing"],
  ["throttle_timing", "Throttle timing"],
  ["input_match", "Pedal match"],
  ["speed", "Speed delta"],
  ["gear", "Gear hint"],
];

const widgetLabels: Array<[keyof WidgetConfig, string]> = [
  ["title", "Title"],
  ["speed_gear_rpm", "Speed, gear, RPM"],
  ["pedals", "Pedals"],
  ["steering", "Steering"],
  ["lap_info", "Lap info"],
  ["lap_timing", "Lap timing"],
  ["sectors", "Sectors"],
  ["mini_sector_widget", "Mini sectors"],
  ["input_history", "Input history"],
  ["delta_timing", "Delta timing"],
  ["ghost_inputs", "Ghost inputs"],
  ["coaching", "Coaching"],
  ["performance_monitor", "Performance monitor"],
];

const legacyWidgetById: Partial<Record<string, keyof WidgetConfig>> = {
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

const layoutLabels: Array<[LayoutWidgetKey, string]> = [
  ["telemetry", "Telemetry"],
  ["inputs", "Inputs"],
  ["lap_timing", "Lap timing"],
  ["timing", "Timing"],
  ["sectors", "Sectors"],
  ["mini_sectors", "Mini sectors"],
  ["coaching", "Coaching"],
  ["performance", "Performance"],
];

const legacySurfaceByWidgetKey: Record<keyof WidgetConfig, LayoutWidgetKey | null> = {
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

const colorLabels: Array<[keyof StyleConfig, string]> = [
  ["background", "Background"],
  ["border", "Border"],
  ["primary_text", "Primary text"],
  ["secondary_text", "Secondary text"],
  ["throttle", "Throttle"],
  ["brake", "Brake"],
  ["clutch", "Clutch"],
  ["steering", "Steering"],
  ["rpm", "RPM"],
  ["delta_gain", "Delta gain"],
  ["delta_loss", "Delta loss"],
  ["delta_neutral", "Delta neutral"],
  ["reference", "Reference"],
  ["coaching_warning", "Coaching warning"],
  ["coaching_positive", "Coaching positive"],
];

function normalizeUiConfig(raw: OverlayConfig): OverlayConfig {
  const incoming = raw as OverlayConfig & { overlays?: OverlayLayerConfig[] };
  if (Array.isArray(incoming.overlays) && incoming.overlays.length > 0) {
    return syncExtraWidgetsWithOverlayMembership(raw);
  }

  const widgetIds = [
    ...enabledLegacySurfaceIds(raw.widgets),
    ...Object.entries(raw.extra_widgets ?? {})
      .filter(([, widget]) => widget.enabled)
      .map(([id]) => id),
  ];
  return syncExtraWidgetsWithOverlayMembership({
    ...raw,
    overlays: [{
      id: "main",
      name: "Main overlay",
      enabled: true,
      window: { ...raw.window },
      widgets: [...new Set(widgetIds)],
      layout_overrides: {},
    }],
  });
}

function App() {
  const [config, setConfig] = useState<OverlayConfig | null>(null);
  const [path, setPath] = useState("");
  const [revision, setRevision] = useState(0);
  const [status, setStatus] = useState("Loading config");
  const [saving, setSaving] = useState(false);
  const [overlayRunning, setOverlayRunning] = useState(false);
  const [overlayEditMode, setOverlayEditMode] = useState(false);
  const [overlayVisible, setOverlayVisible] = useState(true);
  const [copiedWindowGeometry, setCopiedWindowGeometry] = useState(false);
  const [selectedLayout, setSelectedLayout] = useState<LayoutSelection>("telemetry");
  const [layoutSearch, setLayoutSearch] = useState("");
  const [activePage, setActivePage] = useState<Page>("Dashboard");
  const [defaultConfigState, setDefaultConfigState] = useState<OverlayConfig | null>(null);
  const [configText, setConfigText] = useState("");
  const [widgetCatalog, setWidgetCatalog] = useState<WidgetDefinition[]>([]);
  const [widgetSearch, setWidgetSearch] = useState("");
  const [widgetCategory, setWidgetCategory] = useState("All");
  const [activeOverlayId, setActiveOverlayId] = useState("main");
  const [workspace, setWorkspace] = useState<WorkspaceSize>(() => {
    try {
      const saved = window.localStorage.getItem(workspaceStorageKey);
      if (saved) return { ...workspacePresets[0][1], ...JSON.parse(saved) } as WorkspaceSize;
    } catch {
      // Ignore malformed editor-only state and use the safe default.
    }
    return workspacePresets[0][1];
  });
  const hydratedConfig = useRef(false);
  const lastSavedConfig = useRef("");
  const saveTimer = useRef<number | null>(null);

  useEffect(() => {
    void loadConfig();
    void loadDefaultConfig();
    void loadWidgetCatalog();
    void refreshOverlayStatus();
    const id = window.setInterval(() => {
      void refreshOverlayStatus();
    }, 1500);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    if (config && !config.overlays.some((overlay) => overlay.id === activeOverlayId)) {
      setActiveOverlayId(config.overlays[0]?.id ?? "main");
    }
  }, [config, activeOverlayId]);

  useEffect(() => {
    if (!config || !hydratedConfig.current) return;
    const serialized = JSON.stringify(config);
    if (serialized === lastSavedConfig.current) return;
    if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void persistConfig(config, "Changes saved");
      saveTimer.current = null;
    }, 850);
    return () => {
      if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
    };
  }, [config]);

  useEffect(() => {
    window.localStorage.setItem(workspaceStorageKey, JSON.stringify(workspace));
  }, [workspace]);

  async function refreshOverlayStatus(): Promise<boolean> {
    try {
      const running = await invoke<boolean>("overlay_status");
      setOverlayRunning(running);
      if (running) {
        const editing = await invoke<boolean>("overlay_edit_status");
        setOverlayEditMode(editing);
        setOverlayVisible(await invoke<boolean>("overlay_visibility_status"));
      } else {
        setOverlayEditMode(false);
        setOverlayVisible(false);
      }
      return running;
    } catch (error) {
      setStatus(String(error));
      return false;
    }
  }

  async function loadConfig(force = false) {
    if (!force && config && JSON.stringify(config) !== lastSavedConfig.current
      && !window.confirm("You have unsaved changes. Reloading will discard them. Continue?")) {
      return;
    }
    try {
      const response = await invoke<LoadResponse>("load_config");
      const normalized = normalizeUiConfig(response.config);
      setConfig(normalized);
      lastSavedConfig.current = JSON.stringify(normalized);
      hydratedConfig.current = true;
      setPath(response.path);
      setRevision(response.revision);
      setStatus("Config loaded");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function persistConfig(nextConfig: OverlayConfig, successStatus = "Saved"): Promise<boolean> {
    setSaving(true);
    try {
      const response = await invoke<LoadResponse>("save_config", {
        config: nextConfig,
        expected_revision: revision,
      });
      const normalized = normalizeUiConfig(response.config);
      setConfig(normalized);
      lastSavedConfig.current = JSON.stringify(normalized);
      setPath(response.path);
      setRevision(response.revision);
      setStatus(successStatus);
      return true;
    } catch (error) {
      setStatus(String(error));
      return false;
    } finally {
      setSaving(false);
    }
  }

  async function saveConfig(): Promise<boolean> {
    return config ? persistConfig(config) : false;
  }

  async function loadDefaultConfig() {
    try {
      const nextDefault = await invoke<OverlayConfig>("default_config");
      setDefaultConfigState(normalizeUiConfig(nextDefault));
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function exportConfigText() {
    if (!config) {
      return;
    }
    try {
      const text = await invoke<string>("export_config", { config });
      setConfigText(text);
      setStatus("Config exported");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function importConfigText() {
    if (config && JSON.stringify(config) !== lastSavedConfig.current
      && !window.confirm("Importing will replace your pending changes. Continue?")) {
      return;
    }
    try {
      const response = await invoke<LoadResponse>("import_config", {
        text: configText,
        expected_revision: revision,
      });
      setConfig(normalizeUiConfig(response.config));
      setPath(response.path);
      setRevision(response.revision);
      setStatus("Config imported");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function resetConfig() {
    if (!window.confirm("Reset all settings to defaults? This cannot be undone.")) {
      return;
    }
    try {
      const response = await invoke<LoadResponse>("reset_config");
      setConfig(normalizeUiConfig(response.config));
      setPath(response.path);
      setRevision(response.revision);
      setStatus("Config reset");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function startOverlay() {
    try {
      const saved = await saveConfig();
      await invoke("start_overlay");
      const running = await refreshOverlayStatus();
      if (!running) {
        setStatus("HashOverlay did not become ready. Close any older HashOverlay process and try Start overlay again.");
        return;
      }
      setStatus(saved ? "Overlay started" : "Overlay started with the last saved config; current changes were not saved");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function stopOverlay() {
    try {
      await invoke("stop_overlay");
      await refreshOverlayStatus();
      setStatus("Overlay stop requested");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function loadWidgetCatalog() {
    try {
      setWidgetCatalog(await invoke<WidgetDefinition[]>("widget_catalog"));
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function openConfigFolder() {
    try {
      await invoke("open_config_folder");
      setStatus("Config folder opened");
    } catch (error) {
      setStatus(String(error));
    }
  }

  function resetLayout() {
    if (!defaultConfigState) {
      return;
    }
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const arranged = arrangeProfileLayout(currentProfile(current));
      return {
        ...current,
        window: { ...defaultConfigState.window, height: arranged.height },
        layout: arranged.layout,
        extra_widgets: arranged.widgets,
        overlays: current.overlays.map((overlay) => overlay.id === activeOverlayId
          ? {
              ...overlay,
              window: { ...defaultConfigState.window, height: arranged.height },
              layout_overrides: layoutOverridesFromArrangedProfile(arranged.layout, arranged.widgets),
            }
          : overlay),
      };
    });
    setStatus("Layout reset");
  }

  function tidyActiveOverlayLayout() {
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const arranged = arrangeProfileLayout(currentProfile(current));
      const currentOverlay = current.overlays.find((overlay) => overlay.id === activeOverlayId);
      if (!currentOverlay) {
        return current;
      }
      const window = {
        ...currentOverlay.window,
        height: Math.max(currentOverlay.window.height, arranged.height),
      };
      return {
        ...current,
        overlays: current.overlays.map((overlay) => overlay.id === activeOverlayId
          ? {
              ...overlay,
              window,
              layout_overrides: layoutOverridesFromArrangedProfile(arranged.layout, arranged.widgets),
            }
          : overlay),
      };
    });
    setStatus("Overlay layout arranged");
  }

  async function applyPreset(name: StandardPresetKey) {
    if (!config) {
      return;
    }
    const next = applyProfile(config, config.presets[name], activeOverlayId);
    setConfig(next);
    await persistConfig(next, `${titleCase(name)} preset applied`);
  }

  async function applyCustomPresetAndSave(index: number) {
    if (!config) {
      return;
    }
    const next = applyCustomPreset(config, index, activeOverlayId);
    setConfig(next);
    const name = config.presets.custom[index]?.name || `Custom ${index + 1}`;
    await persistConfig(next, `${name} preset applied`);
  }

  async function saveCustomPreset(next: OverlayConfig, successStatus: string) {
    setConfig(next);
    await persistConfig(next, successStatus);
  }

  function setCatalogWidget(id: string, enabled: boolean) {
    const legacy = legacyWidgetById[id];
    const surface = catalogSurfaceId(id);
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const overlays = current.overlays.map((overlay) => overlay.id === activeOverlayId
        ? { ...overlay, widgets: enabled ? [...new Set([...overlay.widgets, surface])] : overlay.widgets.filter((widget) => widget !== surface) }
        : overlay);
      const usedElsewhere = overlayIdUsedByOtherLayer(overlays, activeOverlayId, surface);
      let next = legacy
        ? { ...current, widgets: { ...current.widgets, [legacy]: enabled || usedElsewhere } }
        : current.extra_widgets[id]
          ? { ...current, extra_widgets: { ...current.extra_widgets, [id]: { ...current.extra_widgets[id], enabled: enabled || usedElsewhere } } }
          : current;
      next = {
        ...next,
        overlays,
      };
      return next;
    });
  }

  function addOverlayLayer() {
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const id = nextOverlayId(current.overlays);
      const ordinal = overlayOrdinal(id);
      const layer: OverlayLayerConfig = {
        id,
        name: `Overlay ${ordinal}`,
        enabled: true,
        window: { ...current.window, x: current.window.x + 36, y: current.window.y + 36 },
        widgets: ["coaching"],
        layout_overrides: {},
      };
      setActiveOverlayId(id);
      return { ...current, overlays: [...current.overlays, layer] };
    });
    setStatus("New overlay added");
  }

  async function toggleOverlayEditMode() {
    try {
      const response = await invoke<string>("toggle_overlay_edit_mode");
      const enabled = response === "edit_on";
      setOverlayEditMode(enabled);
      setStatus(enabled ? "Overlay edit mode enabled" : "Overlay edit mode disabled");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function toggleOverlayVisibility() {
    try {
      const visible = await invoke<boolean>("toggle_overlay_visibility");
      setOverlayVisible(visible);
      setStatus(visible ? "Overlay shown" : "Overlay hidden");
    } catch (error) {
      setStatus(String(error));
    }
  }

  function duplicateOverlayLayer(id: string) {
    setConfig((current) => {
      if (!current) return current;
      const source = current.overlays.find((overlay) => overlay.id === id);
      if (!source) return current;
      const nextId = nextOverlayId(current.overlays);
      const duplicate: OverlayLayerConfig = {
        ...source,
        id: nextId,
        name: `${source.name} copy`,
        window: { ...source.window, x: source.window.x + 36, y: source.window.y + 36 },
        widgets: [...source.widgets],
        layout_overrides: Object.fromEntries(
          Object.entries(source.layout_overrides).map(([key, layout]) => [key, { ...layout }]),
        ),
      };
      setActiveOverlayId(nextId);
      return { ...current, overlays: [...current.overlays, duplicate] };
    });
    setStatus("Overlay duplicated");
  }

  function updateOverlayLayer(id: string, update: Partial<OverlayLayerConfig>) {
    setConfig((current) => current ? {
      ...current,
      overlays: current.overlays.map((overlay) => overlay.id === id ? { ...overlay, ...update } : overlay),
    } : current);
  }

  function alignOverlay(id: string, mode: "top-left" | "center") {
    setConfig((current) => {
      if (!current) return current;
      const target = current.overlays.find((overlay) => overlay.id === id);
      if (!target) return current;
      const x = mode === "center" ? workspace.originX + Math.max(0, Math.round((workspace.width - target.window.width) / 2)) : workspace.originX;
      const y = mode === "center" ? workspace.originY + Math.max(0, Math.round((workspace.height - target.window.height) / 2)) : workspace.originY;
      return {
        ...current,
        overlays: current.overlays.map((overlay) => overlay.id === id
          ? { ...overlay, window: { ...overlay.window, x, y } }
          : overlay),
      };
    });
    setStatus(mode === "center" ? "Overlay centered" : "Overlay aligned to top-left");
  }

  function fitOverlaysToWorkspace() {
    setConfig((current) => current ? {
      ...current,
      overlays: current.overlays.map((overlay) => ({
        ...overlay,
        window: {
          ...overlay.window,
          x: Math.max(workspace.originX, Math.min(overlay.window.x, workspace.originX + workspace.width - overlay.window.width)),
          y: Math.max(workspace.originY, Math.min(overlay.window.y, workspace.originY + workspace.height - overlay.window.height)),
        },
      })),
    } : current);
    setStatus("Surfaces fitted to workspace");
  }

  function resetWorkspacePreference() {
    const safeDefault = workspacePresets[0][1];
    window.localStorage.removeItem(workspaceStorageKey);
    setWorkspace(safeDefault);
    setStatus("Workspace reset");
  }

  function removeOverlayLayer(id: string) {
    const target = config?.overlays.find((overlay) => overlay.id === id);
    if (!target || !window.confirm(`Remove “${target.name}”? Its window and widget assignments will be deleted.`)) {
      return;
    }
    setConfig((current) => {
      if (!current || current.overlays.length <= 1) {
        return current;
      }
      const overlays = current.overlays.filter((overlay) => overlay.id !== id);
      setActiveOverlayId(overlays[0]?.id ?? "main");
      return { ...current, overlays };
    });
  }

  const widgetCategories = useMemo(
    () => ["All", ...new Set(widgetCatalog.map((widget) => widget.category))],
    [widgetCatalog],
  );
  const filteredWidgetCatalog = useMemo(
    () => widgetCatalog.filter((widget) => {
      const matchesCategory = widgetCategory === "All" || widget.category === widgetCategory;
      const query = widgetSearch.trim().toLocaleLowerCase();
      return matchesCategory && (!query || `${widget.name} ${widget.description}`.toLocaleLowerCase().includes(query));
    }),
    [widgetCatalog, widgetCategory, widgetSearch],
  );
  const activeOverlayConfig = useMemo(
    () => config ? overlayPreviewConfig(config, activeOverlayId) : null,
    [config, activeOverlayId],
  );
  const activeOverlay = useMemo(
    () => config?.overlays.find((overlay) => overlay.id === activeOverlayId),
    [config, activeOverlayId],
  );
  const availableLayoutEntries = useMemo(() => activeOverlayConfig ? layoutEntries(activeOverlayConfig) : [], [activeOverlayConfig]);
  const filteredLayoutEntries = useMemo(() => {
    const query = layoutSearch.trim().toLocaleLowerCase();
    return availableLayoutEntries.filter((entry) => !query || entry.label.toLocaleLowerCase().includes(query));
  }, [availableLayoutEntries, layoutSearch]);

  useEffect(() => {
    if (!availableLayoutEntries.some((entry) => entry.key === selectedLayout)) {
      setSelectedLayout(availableLayoutEntries[0]?.key ?? "telemetry");
    }
  }, [availableLayoutEntries, selectedLayout]);

  const activePreset = useMemo(() => {
    if (!config) {
      return null;
    }
    const profile = currentProfile(config);
    return presetNames.find((name) => profilesMatch(profile, config.presets[name]));
  }, [config]);

  if (!config) {
    return (
      <main className="shell loading">
        <Activity size={20} />
        <span>{status}</span>
      </main>
    );
  }

  const statusNeedsAttention = /could not|did not|failed|error|conflict|invalid/i.test(status);
  const hasPendingChanges = JSON.stringify(config) !== lastSavedConfig.current;

  return (
    <main className="shell">
      <header className="topbar">
        <div className="brandBlock">
          <div className="brandMark" aria-hidden="true"><span>HO</span><span>01</span></div>
          <div>
            <h1>HashOverlay <span>/ settings</span></h1>
            <p>{path}</p>
          </div>
        </div>
        <div className="topbarRight">
          <div className={`statusPill ${overlayRunning ? "live" : ""} ${overlayRunning && !overlayVisible ? "hiddenState" : ""} ${!overlayRunning && statusNeedsAttention ? "error" : ""}`} title={status} aria-live="polite">
            <span className="statusDot" />
            {overlayRunning ? overlayVisible ? "Overlay live" : "Overlay hidden" : statusNeedsAttention ? "Start needs attention" : "Overlay stopped"}
          </div>
          {!overlayRunning && statusNeedsAttention && <p className="runtimeNotice" role="alert">{status}</p>}
          <div className="actions">
          <button className="iconButton" title="Open config folder" onClick={openConfigFolder}>
            <FolderOpen size={18} />
          </button>
          <button className="iconButton" title={hasPendingChanges ? "Reload config and discard pending changes" : "Reload config"} onClick={() => void loadConfig()}>
            <RotateCcw size={18} />
          </button>
          <button className="primaryButton" onClick={startOverlay} disabled={saving}>
            <Play size={18} />
            Start overlay
          </button>
          <button className="dangerButton" onClick={stopOverlay} disabled={!overlayRunning}>
            <Square size={16} />
            Stop overlay
          </button>
          <button className={`secondaryButton ${overlayEditMode ? "active" : ""}`} onClick={() => void toggleOverlayEditMode()} disabled={!overlayRunning}>
            {overlayEditMode ? "Exit edit mode" : "Edit overlay"}
          </button>
          <button className="secondaryButton" onClick={() => void toggleOverlayVisibility()} disabled={!overlayRunning}>
            {overlayVisible ? "Hide overlay" : "Show overlay"}
          </button>
          <button className="primaryButton" onClick={saveConfig} disabled={saving}>
            <Save size={18} />
            {saving ? "Saving" : "Save"}
          </button>
          <span className={`saveState ${saving ? "saving" : hasPendingChanges ? "pending" : "saved"}`} aria-live="polite">
            {saving ? "Saving changes" : hasPendingChanges ? "Changes pending" : "Auto-saved"}
          </span>
          </div>
        </div>
      </header>

      <section className="sceneBar">
        <div className="sceneHeading">
          <strong>Load a cockpit setup.</strong>
        </div>
        <div className="sceneChoices">
          {presetNames.map((name) => (
            <button
              key={name}
              className={activePreset === name ? "selected" : ""}
              onClick={() => void applyPreset(name)}
              disabled={saving}
            >
              {titleCase(name)}
            </button>
          ))}
          {config.presets.custom.map((preset, index) => (
            <button key={`custom-${index}`} className="customScene" onClick={() => void applyCustomPresetAndSave(index)} disabled={saving}>
              {preset.name || `Custom ${index + 1}`}
            </button>
          ))}
        </div>
      </section>

      <section className="layersBar">
        <div className="sceneHeading">
          <strong>Choose the surface under control.</strong>
        </div>
        <div className="layerChoices">
          {config.overlays.map((overlay) => (
            <button key={overlay.id} className={activeOverlayId === overlay.id ? "selected" : ""} onClick={() => setActiveOverlayId(overlay.id)}>
              <span>{overlay.name}</span>
              <small>{overlay.enabled ? "ON" : "OFF"} · {overlay.widgets.length} widgets · {overlay.window.x},{overlay.window.y} · {overlay.window.width}×{overlay.window.height}</small>
            </button>
          ))}
          <button className="addLayerButton" onClick={addOverlayLayer}><Plus size={15} /> Add overlay</button>
        </div>
        {config.overlays.find((overlay) => overlay.id === activeOverlayId) && (() => {
          const overlay = config.overlays.find((item) => item.id === activeOverlayId)!;
          return (
            <div className="layerControls">
              <input
                value={overlay.name}
                onChange={(event) => updateOverlayLayer(overlay.id, { name: event.target.value })}
                onBlur={(event) => updateOverlayLayer(overlay.id, { name: event.target.value.trim() || "Unnamed overlay" })}
                aria-label="Overlay name"
                placeholder="Overlay name"
              />
              <label className="toggle"><input type="checkbox" checked={overlay.enabled} onChange={(event) => updateOverlayLayer(overlay.id, { enabled: event.target.checked })} /><span>Run this overlay</span></label>
              <button className="secondaryButton" onClick={() => duplicateOverlayLayer(overlay.id)}><Copy size={15} /> Duplicate</button>
              <button className="secondaryButton" onClick={() => alignOverlay(overlay.id, "top-left")}>Top-left</button>
              <button className="secondaryButton" onClick={() => alignOverlay(overlay.id, "center")}>Center</button>
              <button className="dangerButton" onClick={() => removeOverlayLayer(overlay.id)} disabled={config.overlays.length <= 1}><Trash2 size={15} /> Remove</button>
            </div>
          );
        })()}
      </section>

      <section className="controlStrip" aria-label="Active overlay summary">
        <div className="controlLead">
          <span className="stripLabel">Armed surface</span>
          <strong>{activeOverlay?.name ?? "Main overlay"}</strong>
        </div>
        <div className="stripMetric">
          <span className="stripLabel">Widgets</span>
          <strong>{activeOverlay?.widgets.length ?? 0}</strong>
        </div>
        <div className="stripMetric">
          <span className="stripLabel">Window</span>
          <strong>{(activeOverlayConfig ?? config).window.width} × {(activeOverlayConfig ?? config).window.height}</strong>
        </div>
        <div className="stripMetric">
          <span className="stripLabel">Deployment</span>
          <strong className={activeOverlay?.enabled ? "valueLive" : "valueMuted"}>{activeOverlay?.enabled ? "Armed" : "Standby"}</strong>
        </div>
        <p className="stripHint">Pick a surface, then set its layout and live data modules.</p>
      </section>

      <nav className="tabs">
        {pages.map((page) => (
          <button key={page} className={activePage === page ? "selected" : ""} onClick={() => setActivePage(page)}>
            {page}
          </button>
        ))}
      </nav>

      <div className="grid">
        {showPanel(activePage, "Dashboard", "Layout") && <section className="layoutStudio">
          <Section icon={<Activity />} title="Live Preview">
            <p className="previewHint">Select a surface on the screen map, drag its window to place it, then arrange its widgets below.</p>
            <SurfaceMap
              config={config}
              selected={activeOverlayId}
              onSelect={setActiveOverlayId}
              workspace={workspace}
              onWorkspaceChange={setWorkspace}
              onFitToWorkspace={fitOverlaysToWorkspace}
              onCenterSelected={() => alignOverlay(activeOverlayId, "center")}
              onResetWorkspace={resetWorkspacePreference}
              onMove={(id, x, y) => setOverlayPosition(config, setConfig, id, x, y)}
            />
            <OverlayPreview
              config={activeOverlayConfig ?? config}
              selected={selectedLayout}
              onSelect={setSelectedLayout}
              onLayoutChange={(widget, layout) => setConfig(updateOverlayLayoutSelection(config, activeOverlayId, widget, layout))}
              onTidyLayout={tidyActiveOverlayLayout}
            />
          </Section>
          <aside className="layoutInspector">
            <Section icon={<LayoutGrid />} title="Overlay Window">
              <p className="previewHint">This is the real screen position of the selected surface. Set it here, or use F10 in-game for direct adjustment.</p>
              <button className="secondaryButton geometryCopyButton" onClick={() => {
                const windowConfig = (activeOverlayConfig ?? config).window;
                void navigator.clipboard?.writeText(`x=${windowConfig.x}, y=${windowConfig.y}, width=${windowConfig.width}, height=${windowConfig.height}`);
                setCopiedWindowGeometry(true);
                window.setTimeout(() => setCopiedWindowGeometry(false), 1400);
                setStatus("Window geometry copied");
              }}>{copiedWindowGeometry ? "Copied" : "Copy window geometry"}</button>
              <div className="windowPositionFields">
                <NumberField label="Screen X" value={(activeOverlayConfig ?? config).window.x} onChange={(value) => setOverlayWindow(config, setConfig, activeOverlayId, "x", value)} />
                <NumberField label="Screen Y" value={(activeOverlayConfig ?? config).window.y} onChange={(value) => setOverlayWindow(config, setConfig, activeOverlayId, "y", value)} />
              </div>
              <NumberField label="Width" value={(activeOverlayConfig ?? config).window.width} onChange={(value) => setOverlayWindow(config, setConfig, activeOverlayId, "width", value)} />
              <NumberField label="Height" value={(activeOverlayConfig ?? config).window.height} onChange={(value) => setOverlayWindow(config, setConfig, activeOverlayId, "height", value)} />
              <RangeField label="Scale" min={0.65} max={1.75} step={0.05} value={config.style.scale} onChange={(value) => setStyle(config, setConfig, "scale", value)} />
              <RangeField label="Opacity" min={32} max={255} step={1} value={config.style.opacity} onChange={(value) => setStyle(config, setConfig, "opacity", value)} />
            </Section>
            <Section icon={<Magnet />} title="Selected Widget">
          <div className="layoutPickerHeader">
            <input
              value={layoutSearch}
              onChange={(event) => setLayoutSearch(event.target.value)}
              placeholder="Find widget..."
              aria-label="Find widget"
            />
            <span>{filteredLayoutEntries.length}/{availableLayoutEntries.length}</span>
          </div>
          <div className="segmented">
            {filteredLayoutEntries.map(({ key, label }) => (
              <button key={key} className={selectedLayout === key ? "selected" : ""} onClick={() => setSelectedLayout(key)}>
                {label}
              </button>
            ))}
          </div>
          {filteredLayoutEntries.length === 0 && <p className="emptyHint">No widgets match this search.</p>}
          <label className="toggle full">
            <input
              type="checkbox"
              checked={config.layout.lock_all}
              onChange={(event) => setLayoutFlag(config, setConfig, "lock_all", event.target.checked)}
            />
            <span>Lock all widgets</span>
          </label>
          <label className="toggle full">
            <input
              type="checkbox"
              checked={config.layout.snap_to_edges}
              onChange={(event) => setLayoutFlag(config, setConfig, "snap_to_edges", event.target.checked)}
            />
            <span>Snap to screen edges</span>
          </label>
          <label className="toggle full">
            <input
              type="checkbox"
              checked={config.layout.snap_to_grid}
              onChange={(event) => setLayoutFlag(config, setConfig, "snap_to_grid", event.target.checked)}
            />
            <span>Snap to grid</span>
          </label>
          <label className="toggle full">
            <input
              type="checkbox"
              checked={config.layout.snap_to_widgets}
              onChange={(event) => setLayoutFlag(config, setConfig, "snap_to_widgets", event.target.checked)}
            />
            <span>Snap to widgets</span>
          </label>
          <Segmented value={String(config.layout.grid_size)} options={gridSizes} onChange={(value) => setLayoutFlag(config, setConfig, "grid_size", Number(value))} />
          <RangeField label="Snap distance" min={0} max={64} step={1} value={config.layout.snap_distance} onChange={(value) => setLayoutFlag(config, setConfig, "snap_distance", value)} />
          <div className="selectionSummary">
            <div>
              <span className="summaryLabel">Selected geometry</span>
              <strong>{Math.round(layoutForSelection(activeOverlayConfig ?? config, selectedLayout).x)}, {Math.round(layoutForSelection(activeOverlayConfig ?? config, selectedLayout).y)}</strong>
              <span>position</span>
            </div>
            <div>
              <strong>{Math.round(layoutForSelection(activeOverlayConfig ?? config, selectedLayout).width)} × {Math.round(layoutForSelection(activeOverlayConfig ?? config, selectedLayout).height)}</strong>
              <span>size</span>
            </div>
            <p>Drag the widget in the preview or use the shortcuts below to place it precisely.</p>
          </div>
          <WidgetLayoutFields
            layout={layoutForSelection(activeOverlayConfig ?? config, selectedLayout)}
            defaultLayout={layoutForSelection(defaultConfigState ?? activeOverlayConfig ?? config, selectedLayout)}
            windowWidth={(activeOverlayConfig ?? config).window.width}
            windowHeight={(activeOverlayConfig ?? config).window.height}
            onChange={(key, value) => setOverlayLayoutSelection(config, setConfig, activeOverlayId, selectedLayout, key, value)}
            onReplace={(layout) => setConfig(updateOverlayLayoutSelection(
              config,
              activeOverlayId,
              selectedLayout,
              normalizeWidgetLayoutForWindow((activeOverlayConfig ?? config).window, layout),
            ))}
          />
          {selectedLayout.startsWith("extra:") && config.extra_widgets[selectedLayout.slice("extra:".length)] && (
            <>
              <WidgetStyleFields
                style={config.extra_widgets[selectedLayout.slice("extra:".length)].style}
                onChange={(key, value) => setExtraWidgetStyle(config, setConfig, selectedLayout.slice("extra:".length), key, value)}
              />
              <WidgetOptionsFields
                id={selectedLayout.slice("extra:".length)}
                options={config.extra_widgets[selectedLayout.slice("extra:".length)].options}
                onChange={(key, value) => setExtraWidgetOption(config, setConfig, selectedLayout.slice("extra:".length), key, value)}
              />
            </>
          )}
            </Section>
          </aside>
        </section>}

        {showPanel(activePage, "Dashboard", "Performance") && <Section icon={<Gauge />} title="Performance">
          <Segmented value={config.performance.mode} options={performanceModes} onChange={(value) => setPerformance(config, setConfig, value)} />
          <NumberField label="Render FPS" value={config.window.refresh_hz} onChange={(value) => setPerformanceWindow(config, setConfig, "refresh_hz", value)} />
          <NumberField label="Sample ms" value={config.window.sample_ms} onChange={(value) => setPerformanceWindow(config, setConfig, "sample_ms", value)} />
          <NumberField label="History samples" value={config.window.history_samples} onChange={(value) => setPerformanceWindow(config, setConfig, "history_samples", value)} />
          <RangeField label="Line thickness" min={1} max={8} step={1} value={config.style.line_thickness} onChange={(value) => setStyle(config, setConfig, "line_thickness", value)} />
        </Section>}

        {showPanel(activePage, "Dashboard", "Timing") && <Section icon={<Timer />} title="Timing">
          <Segmented value={config.timing.reference_mode} options={referenceModes} onChange={(value) => setTiming(config, setConfig, "reference_mode", value)} />
          <NumberField label="Mini sectors" value={config.timing.mini_sectors} onChange={(value) => setTiming(config, setConfig, "mini_sectors", value)} />
          <RangeField label="Brake threshold" min={0.01} max={1} step={0.01} value={config.timing.brake_threshold} onChange={(value) => setTiming(config, setConfig, "brake_threshold", value)} />
          <RangeField label="Throttle threshold" min={0.01} max={1} step={0.01} value={config.timing.throttle_threshold} onChange={(value) => setTiming(config, setConfig, "throttle_threshold", value)} />
        </Section>}

        {showPanel(activePage, "Dashboard", "Coaching") && <Section icon={<Activity />} title="Coaching">
          <Segmented value={config.coaching.mode} options={coachingModes} onChange={(value) => setCoaching(config, setConfig, "mode", value)} />
          <RangeField label="Speed threshold" min={1} max={40} step={1} value={config.coaching.speed_threshold_kph} onChange={(value) => setCoaching(config, setConfig, "speed_threshold_kph", value)} />
          <RangeField label="Timing deadband" min={0} max={50} step={1} value={config.coaching.timing_deadband_m} onChange={(value) => setCoaching(config, setConfig, "timing_deadband_m", value)} />
          <RangeField label="Corner match tolerance" min={10} max={500} step={5} value={config.coaching.event_match_tolerance_m} onChange={(value) => setCoaching(config, setConfig, "event_match_tolerance_m", value)} />
          <NumberField label="Max hints" value={config.coaching.max_hints} onChange={(value) => setCoaching(config, setConfig, "max_hints", value)} />
          <div className="toggles">
            {coachingLabels.map(([key, label]) => (
              <label className="toggle" key={key}>
                <input
                  type="checkbox"
                  checked={config.coaching[key]}
                  onChange={(event) => setCoaching(config, setConfig, key, event.target.checked)}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
        </Section>}

        {showPanel(activePage, "Dashboard", "Widgets") && <Section icon={<SlidersHorizontal />} title="Widgets">
          <div className="toggles">
            {widgetLabels.map(([key, label]) => (
              <label className="toggle" key={key}>
                <input
                  type="checkbox"
                  checked={activeLegacyWidgetEnabled(config, activeOverlayId, key)}
                  onChange={(event) => setWidget(config, setConfig, activeOverlayId, key, event.target.checked)}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>
          <div className="widgetBrowser">
            <input value={widgetSearch} placeholder="Search widgets" onChange={(event) => setWidgetSearch(event.target.value)} />
            <div className="segmented">
              {widgetCategories.map((category) => (
                <button key={category} className={widgetCategory === category ? "selected" : ""} onClick={() => setWidgetCategory(category)}>{category}</button>
              ))}
            </div>
            {filteredWidgetCatalog.map((widget) => {
              const legacy = legacyWidgetById[widget.id];
              const activeOverlay = config.overlays.find((overlay) => overlay.id === activeOverlayId);
              const surface = catalogSurfaceId(widget.id);
              const globallyEnabled = legacy ? activeLegacyWidgetEnabled(config, activeOverlayId, legacy) : config.extra_widgets[widget.id]?.enabled ?? false;
              const enabled = activeOverlay
                ? activeOverlay.widgets.includes(surface) && globallyEnabled
                : globallyEnabled;
              return (
                <label className="widgetCatalogRow" key={widget.id}>
                  <input type="checkbox" checked={enabled} disabled={widget.status === "Unavailable in current LMU interface"} onChange={(event) => setCatalogWidget(widget.id, event.target.checked)} />
                  <span><strong>{widget.name}</strong><small>{widget.description} · {widget.data_requirement} · {widget.status}</small></span>
                </label>
              );
            })}
          </div>
        </Section>}

        {showPanel(activePage, "Dashboard", "Appearance") && <Section icon={<Paintbrush />} title="Appearance">
          <Segmented value={config.style.theme} options={themes} onChange={(value) => setConfig(applyTheme(config, value))} />
          <Segmented value={config.units.speed} options={speedUnits} onChange={(value) => setUnits(config, setConfig, "speed", value)} />
          <Segmented value={config.units.temperature} options={temperatureUnits} onChange={(value) => setUnits(config, setConfig, "temperature", value)} />
          <Segmented value={config.units.pressure} options={pressureUnits} onChange={(value) => setUnits(config, setConfig, "pressure", value)} />
          <Segmented value={config.units.fuel} options={fuelUnits} onChange={(value) => setUnits(config, setConfig, "fuel", value)} />
          <TextInput label="Font family" value={config.style.font_family} onChange={(value) => setStyle(config, setConfig, "font_family", value)} />
          <RangeField label="Font size" min={8} max={36} step={1} value={config.style.font_size} onChange={(value) => setStyle(config, setConfig, "font_size", value)} />
          <RangeField label="Font weight" min={100} max={900} step={100} value={config.style.font_weight} onChange={(value) => setStyle(config, setConfig, "font_weight", value)} />
          <RangeField label="Large number size" min={12} max={72} step={1} value={config.style.large_number_size} onChange={(value) => setStyle(config, setConfig, "large_number_size", value)} />
          <div className="swatches">
            {colorLabels.map(([key, label]) => (
              <label className="swatch" key={key}>
                <span>{label}</span>
                <input
                  type="color"
                  value={String(config.style[key])}
                  onChange={(event) => setStyle(config, setConfig, key, event.target.value)}
                />
              </label>
            ))}
          </div>
        </Section>}

        {showPanel(activePage, "Dashboard", "Hotkeys") && <Section icon={<Activity />} title="Hotkeys">
          <HotkeyField label="Show or hide overlay" value={config.hotkeys.toggle_overlay} conflict={hasHotkeyConflict(config.hotkeys, "toggle_overlay")} onChange={(value) => setHotkey(config, setConfig, "toggle_overlay", value)} />
          <HotkeyField label="Edit mode" value={config.hotkeys.edit_mode} conflict={hasHotkeyConflict(config.hotkeys, "edit_mode")} onChange={(value) => setHotkey(config, setConfig, "edit_mode", value)} />
          <HotkeyField label="Toggle coaching" value={config.hotkeys.toggle_coaching} conflict={hasHotkeyConflict(config.hotkeys, "toggle_coaching")} onChange={(value) => setHotkey(config, setConfig, "toggle_coaching", value)} />
          <HotkeyField label="Cycle preset" value={config.hotkeys.cycle_preset} conflict={hasHotkeyConflict(config.hotkeys, "cycle_preset")} onChange={(value) => setHotkey(config, setConfig, "cycle_preset", value)} />
        </Section>}

        {showPanel(activePage, "Dashboard", "Presets") && <Section icon={<Plus />} title="Custom Presets">
          <button className="primaryButton wide" onClick={() => void saveCustomPreset(addCustomPreset(config), "Custom preset created")} disabled={saving}>
            <Plus size={18} />
            New from current
          </button>
          <div className="presetList">
            {config.presets.custom.map((preset, index) => (
              <div className="presetItem" key={`${preset.name}-${index}`}>
                <input
                  value={preset.name}
                  onChange={(event) => setConfig(renameCustomPreset(config, index, event.target.value))}
                />
                <button title="Apply preset" onClick={() => void applyCustomPresetAndSave(index)} disabled={saving}>
                  Apply
                </button>
                <button title="Save current into preset" onClick={() => void saveCustomPreset(saveCurrentIntoCustomPreset(config, index), "Custom preset updated")} disabled={saving}>
                  <Save size={16} />
                </button>
                <button title="Delete preset" onClick={() => void saveCustomPreset(deleteCustomPreset(config, index), "Custom preset deleted")} disabled={saving}>
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>
        </Section>}

        {showPanel(activePage, "Advanced", "Advanced") && <Section icon={<Save />} title="Import and Export">
          <div className="buttonRow">
            <button className="primaryButton" onClick={exportConfigText}>
              Export
            </button>
            <button className="primaryButton" onClick={importConfigText} disabled={!configText.trim()}>
              Import
            </button>
            <button className="secondaryButton" onClick={() => setConfigText("")} disabled={!configText}>
              Clear text
            </button>
            <button className="primaryButton" onClick={resetLayout} disabled={!defaultConfigState}>
              Reset layout
            </button>
            <button className="dangerButton" onClick={resetConfig}>
              Reset all
            </button>
          </div>
          <label className="field stack">
            <span className="textAreaLabel">Config TOML <small>{configText.length.toLocaleString()} characters</small></span>
            <textarea value={configText} onChange={(event) => setConfigText(event.target.value)} spellCheck={false} placeholder="Export a configuration or paste TOML here before importing." />
          </label>
          <p className="previewHint">Import replaces the current settings after validation. Export first if you want a backup.</p>
        </Section>}
      </div>

      <footer className="status">{status}</footer>
    </main>
  );
}

function SurfaceMap(props: {
  config: OverlayConfig;
  selected: string;
  workspace: WorkspaceSize;
  onWorkspaceChange: (size: WorkspaceSize) => void;
  onFitToWorkspace: () => void;
  onCenterSelected: () => void;
  onResetWorkspace: () => void;
  onSelect: (id: string) => void;
  onMove: (id: string, x: number, y: number) => void;
}) {
  const [drag, setDrag] = useState<{ id: string; startX: number; startY: number; x: number; y: number } | null>(null);
  const [copiedGeometry, setCopiedGeometry] = useState(false);
  const [customWorkspace, setCustomWorkspace] = useState(props.workspace);
  const mapRef = useRef<HTMLDivElement>(null);
  const [canvasWidth, setCanvasWidth] = useState(900);
  const canvasHeight = 506;
  const virtualWidth = props.workspace.width;
  const virtualHeight = props.workspace.height;
  const scale = canvasWidth / virtualWidth;
  const outOfBounds = props.config.overlays.filter((overlay) =>
    overlay.window.x < props.workspace.originX || overlay.window.y < props.workspace.originY
      || overlay.window.x + overlay.window.width > props.workspace.originX + virtualWidth
      || overlay.window.y + overlay.window.height > props.workspace.originY + virtualHeight,
    ).length;
  const selectedOverlay = props.config.overlays.find((overlay) => overlay.id === props.selected);

  useEffect(() => {
    if (!mapRef.current) return;
    const updateSize = () => setCanvasWidth(mapRef.current?.clientWidth || 900);
    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(mapRef.current);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    setCustomWorkspace(props.workspace);
  }, [props.workspace.width, props.workspace.height, props.workspace.originX, props.workspace.originY]);

  function move(clientX: number, clientY: number) {
    if (!drag) return;
    const overlay = props.config.overlays.find((item) => item.id === drag.id);
    if (!overlay) return;
    const nextX = Math.round(drag.x + (clientX - drag.startX) / scale);
    const nextY = Math.round(drag.y + (clientY - drag.startY) / scale);
    props.onMove(
      drag.id,
          clamp(nextX, props.workspace.originX, props.workspace.originX + Math.max(0, virtualWidth - overlay.window.width)),
          clamp(nextY, props.workspace.originY, props.workspace.originY + Math.max(0, virtualHeight - overlay.window.height)),
    );
  }

  return (
    <div className="surfaceMapFrame">
      <div className="surfaceMapHeader">
        <span>Screen workspace · {virtualWidth} × {virtualHeight} · origin {props.workspace.originX}, {props.workspace.originY}</span>
        <span className={outOfBounds > 0 ? "mapWarning" : "mapHint"}>
          {outOfBounds > 0 ? `${outOfBounds} surface${outOfBounds === 1 ? "" : "s"} outside workspace` : "Drag surfaces to position"}
        </span>
        <div className="surfaceMapActions">
          <button className="mapFitButton" onClick={props.onCenterSelected} disabled={!selectedOverlay}>Center selected</button>
          {outOfBounds > 0 && <button className="mapFitButton" onClick={props.onFitToWorkspace}>Fit surfaces</button>}
        </div>
      </div>
      <div className="workspacePresets" aria-label="Workspace resolution">
        {workspacePresets.map(([label, size]) => (
          <button
            key={label}
            className={size.width === virtualWidth && size.height === virtualHeight && size.originX === props.workspace.originX && size.originY === props.workspace.originY ? "selected" : ""}
            onClick={() => {
              setCustomWorkspace(size);
              props.onWorkspaceChange(size);
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div className="customWorkspace">
        <label>Width <input aria-label="Workspace width" type="number" min="320" max="16384" value={customWorkspace.width} onChange={(event) => setCustomWorkspace({ ...customWorkspace, width: Number(event.target.value) })} /></label>
        <label>Height <input aria-label="Workspace height" type="number" min="240" max="8640" value={customWorkspace.height} onChange={(event) => setCustomWorkspace({ ...customWorkspace, height: Number(event.target.value) })} /></label>
        <label>Origin X <input aria-label="Workspace origin X" type="number" value={customWorkspace.originX} onChange={(event) => setCustomWorkspace({ ...customWorkspace, originX: Number(event.target.value) })} /></label>
        <label>Origin Y <input aria-label="Workspace origin Y" type="number" value={customWorkspace.originY} onChange={(event) => setCustomWorkspace({ ...customWorkspace, originY: Number(event.target.value) })} /></label>
        <button onClick={() => props.onWorkspaceChange({
          width: boundedInteger(customWorkspace.width, 320, 16384, virtualWidth),
          height: boundedInteger(customWorkspace.height, 240, 8640, virtualHeight),
          originX: finiteInteger(customWorkspace.originX, props.workspace.originX),
          originY: finiteInteger(customWorkspace.originY, props.workspace.originY),
        })}>Use custom</button>
        <button className="workspaceResetButton" onClick={props.onResetWorkspace}>Reset editor</button>
      </div>
      <div
        className="surfaceMap"
        ref={mapRef}
        onPointerMove={(event) => move(event.clientX, event.clientY)}
        onPointerUp={() => setDrag(null)}
        onPointerCancel={() => setDrag(null)}
        style={{ aspectRatio: `${canvasWidth} / ${canvasHeight}` }}
      >
        {props.config.overlays.map((overlay) => (
          <button
            key={overlay.id}
            className={`surfaceMapItem ${props.selected === overlay.id ? "selected" : ""} ${!overlay.enabled ? "disabled" : ""}`}
            style={{
              left: (overlay.window.x - props.workspace.originX) * scale,
              top: (overlay.window.y - props.workspace.originY) * scale,
              width: Math.max(58, overlay.window.width * scale),
              height: Math.max(30, overlay.window.height * scale),
            }}
            onClick={() => props.onSelect(overlay.id)}
            onPointerDown={(event) => {
              props.onSelect(overlay.id);
              setDrag({ id: overlay.id, startX: event.clientX, startY: event.clientY, x: overlay.window.x, y: overlay.window.y });
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            aria-pressed={props.selected === overlay.id}
            onKeyDown={(event) => {
              const step = event.shiftKey ? 10 : 1;
              let dx = 0;
              let dy = 0;
              if (event.key === "ArrowLeft") dx = -step;
              if (event.key === "ArrowRight") dx = step;
              if (event.key === "ArrowUp") dy = -step;
              if (event.key === "ArrowDown") dy = step;
              if (dx === 0 && dy === 0) return;
              event.preventDefault();
              props.onMove(
                overlay.id,
                clamp(overlay.window.x + dx, props.workspace.originX, props.workspace.originX + Math.max(0, virtualWidth - overlay.window.width)),
                clamp(overlay.window.y + dy, props.workspace.originY, props.workspace.originY + Math.max(0, virtualHeight - overlay.window.height)),
              );
            }}
            aria-label={`${overlay.name}, position ${overlay.window.x}, ${overlay.window.y}. Use arrow keys to move.`}
            title={`${overlay.name} · ${overlay.window.x}, ${overlay.window.y}`}
          >
            <strong>{overlay.name}</strong>
            <small>{overlay.window.x}, {overlay.window.y}</small>
          </button>
        ))}
      </div>
      {selectedOverlay && (
        <div className="surfaceMapSelection" aria-live="polite">
          <strong>{selectedOverlay.name}</strong>
          <span>{selectedOverlay.window.x}, {selectedOverlay.window.y}</span>
          <span>{selectedOverlay.window.width} × {selectedOverlay.window.height}</span>
          <span className={selectedOverlay.enabled ? "valueLive" : "valueMuted"}>{selectedOverlay.enabled ? "Enabled" : "Disabled"}</span>
          <button className="mapFitButton" onClick={() => {
            void navigator.clipboard?.writeText(`${selectedOverlay.name}: x=${selectedOverlay.window.x}, y=${selectedOverlay.window.y}, width=${selectedOverlay.window.width}, height=${selectedOverlay.window.height}`);
            setCopiedGeometry(true);
            window.setTimeout(() => setCopiedGeometry(false), 1400);
          }}>{copiedGeometry ? "Copied" : "Copy geometry"}</button>
          <small>Focus a surface and use arrow keys to move it; hold Shift for 10 px.</small>
        </div>
      )}
    </div>
  );
}

function showPanel(active: Page, primary: Page, secondary: Page) {
  return active === "Dashboard" || active === primary || active === secondary;
}

function OverlayPreview(props: {
  config: OverlayConfig;
  selected: LayoutSelection;
  onSelect: (value: LayoutSelection) => void;
  onLayoutChange: (widget: LayoutSelection, layout: WidgetLayout) => void;
  onTidyLayout: () => void;
}) {
  const [drag, setDrag] = useState<{
    widget: LayoutSelection;
    startX: number;
    startY: number;
    layout: WidgetLayout;
    resize: boolean;
  } | null>(null);
  const maxPreviewWidth = 760;
  const maxPreviewHeight = 680;
  const scale = Math.min(
    1.45,
    maxPreviewWidth / props.config.window.width,
    maxPreviewHeight / props.config.window.height,
  );
  const previewWidth = props.config.window.width * scale;
  const previewHeight = props.config.window.height * scale;
  const collisions = layoutCollisions(props.config);

  function moveWidget(clientX: number, clientY: number) {
    if (!drag) {
      return;
    }
    const deltaX = (clientX - drag.startX) / scale;
    const deltaY = (clientY - drag.startY) / scale;
    const next = drag.resize
      ? {
          ...drag.layout,
          width: clamp(Math.round(drag.layout.width + deltaX / clamp(drag.layout.scale, 0.5, 2)), 48, Math.floor(props.config.window.width / clamp(drag.layout.scale, 0.5, 2))),
          height: clamp(Math.round(drag.layout.height + deltaY / clamp(drag.layout.scale, 0.5, 2)), 20, Math.floor(props.config.window.height / clamp(drag.layout.scale, 0.5, 2))),
        }
      : {
          ...drag.layout,
          x: Math.round(drag.layout.x + deltaX),
          y: Math.round(drag.layout.y + deltaY),
        };
    props.onLayoutChange(drag.widget, snapLayout(props.config, drag.widget, next));
  }

  return (
    <div className="previewFrame">
      <div className="previewStatus" aria-live="polite">
        <span>{Math.round(scale * 100)}% workspace scale</span>
        <span className={collisions.length > 0 ? "layoutWarning" : "layoutClear"}>
          {collisions.length > 0 ? `${collisions.length} overlap${collisions.length === 1 ? "" : "s"} detected` : "Layout clear"}
        </span>
        {collisions.length > 0 && <button className="tidyLayoutButton" title={collisions.map(([first, second]) => `${first} × ${second}`).join("\n")} onClick={props.onTidyLayout}>Arrange widgets</button>}
      </div>
      {collisions.length > 0 && <p className="collisionDetails">{collisions.slice(0, 3).map(([first, second]) => `${first} × ${second}`).join(" · ")}{collisions.length > 3 ? " · …" : ""}</p>}
      <div className="previewShortcuts" aria-label="Preview keyboard shortcuts">
        <span><kbd>Click</kbd> select</span>
        <span><kbd>Drag</kbd> move</span>
        <span><kbd>Corner</kbd> resize</span>
        <span><kbd>↑ ↓ ← →</kbd> nudge</span>
        <span><kbd>Shift</kbd> 10 px</span>
        {props.config.layout.lock_all && <strong>Editing locked</strong>}
      </div>
      <div className="previewWrap">
        <div
        className="preview"
        onPointerMove={(event) => moveWidget(event.clientX, event.clientY)}
        onPointerUp={() => setDrag(null)}
        onPointerCancel={() => setDrag(null)}
        style={{
          width: previewWidth,
          height: previewHeight,
          backgroundColor: props.config.style.background,
          borderColor: props.config.style.border,
          color: props.config.style.primary_text,
        }}
      >
        {layoutEntries(props.config).map(({ key, label, layout }) => {
          const widgetStyle = key.startsWith("extra:") ? props.config.extra_widgets[key.slice("extra:".length)]?.style : undefined;
          const useCustomStyle = widgetStyle && !widgetStyle.inherit_theme;
          return (
            <button
              key={key}
              className={`previewWidget ${props.selected === key ? "selected" : ""} ${layout.locked ? "locked" : ""}`}
              style={{
                left: layout.x * scale,
                top: layout.y * scale,
                width: layout.width * scale,
                height: layout.height * scale,
                opacity: layout.opacity,
                transform: `scale(${layout.scale})`,
                transformOrigin: "top left",
                zIndex: layout.z_index,
                backgroundColor: useCustomStyle && widgetStyle.show_background ? widgetStyle.background_color : undefined,
                borderColor: useCustomStyle && widgetStyle.show_border ? widgetStyle.border_color : props.config.style.border,
                color: useCustomStyle ? widgetStyle.primary_color : undefined,
              }}
              onClick={() => props.onSelect(key)}
              onPointerDown={(event) => {
                props.onSelect(key);
                if (props.config.layout.lock_all || layout.locked) {
                  return;
                }
                const resize =
                  event.nativeEvent.offsetX >= layout.width * scale - 14 &&
                  event.nativeEvent.offsetY >= layout.height * scale - 14;
                setDrag({
                  widget: key,
                  startX: event.clientX,
                  startY: event.clientY,
                  layout,
                  resize,
                });
                event.currentTarget.setPointerCapture(event.pointerId);
              }}
              onKeyDown={(event) => {
                if (props.config.layout.lock_all || layout.locked) return;
                const step = event.shiftKey ? 10 : 1;
                let dx = 0;
                let dy = 0;
                if (event.key === "ArrowLeft") dx = -step;
                if (event.key === "ArrowRight") dx = step;
                if (event.key === "ArrowUp") dy = -step;
                if (event.key === "ArrowDown") dy = step;
                if (dx === 0 && dy === 0) return;
                event.preventDefault();
                props.onSelect(key);
                props.onLayoutChange(key, snapLayout(props.config, key, {
                  ...layout,
                  x: layout.x + dx,
                  y: layout.y + dy,
                }));
              }}
              aria-label={`${label}, position ${layout.x}, ${layout.y}${layout.locked ? ". Locked." : ". Use arrow keys to move."}`}
              title={layout.locked ? `${label} · locked` : label}
            >
              <span>{label}</span>
            </button>
          );
        })}
        </div>
      </div>
    </div>
  );
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function finiteInteger(value: number, fallback: number) {
  return Number.isFinite(value) ? Math.round(value) : fallback;
}

function boundedInteger(value: number, min: number, max: number, fallback: number) {
  return clamp(finiteInteger(value, fallback), min, max);
}

function layoutEntries(config: OverlayConfig): Array<{ key: LayoutSelection; label: string; layout: WidgetLayout }> {
  const legacy = layoutLabels
    .filter(([key]) => legacySurfaceEnabled(config.widgets, key))
    .map(([key, label]) => ({ key, label, layout: config.layout[key] }));
  const extra = Object.entries(config.extra_widgets)
    .filter(([, widget]) => widget.enabled)
    .map(([id, widget]) => ({ key: `extra:${id}` as LayoutSelection, label: id.replace(/_/g, " "), layout: widget.layout }));
  return [...legacy, ...extra];
}

function layoutCollisions(config: OverlayConfig) {
  const entries = layoutEntries(config);
  const collisions: Array<[string, string]> = [];
  for (let left = 0; left < entries.length; left += 1) {
    for (let right = left + 1; right < entries.length; right += 1) {
      const first = entries[left];
      const second = entries[right];
      const firstWidth = scaledDimension(first.layout.width, first.layout.scale);
      const firstHeight = scaledDimension(first.layout.height, first.layout.scale);
      const secondWidth = scaledDimension(second.layout.width, second.layout.scale);
      const secondHeight = scaledDimension(second.layout.height, second.layout.scale);
      const overlaps = first.layout.x < second.layout.x + secondWidth
        && first.layout.x + firstWidth > second.layout.x
        && first.layout.y < second.layout.y + secondHeight
        && first.layout.y + firstHeight > second.layout.y;
      if (overlaps) {
        collisions.push([first.label, second.label]);
      }
    }
  }
  return collisions;
}

function overlayPreviewConfig(config: OverlayConfig, overlayId: string): OverlayConfig {
  const layer = config.overlays.find((overlay) => overlay.id === overlayId);
  if (!layer) {
    return config;
  }
  const next = structuredClone(config);
  next.window = { ...layer.window };
  const has = (id: string) => layer.widgets.includes(id);
  next.widgets.title = next.widgets.title && has("telemetry");
  next.widgets.speed_gear_rpm = next.widgets.speed_gear_rpm && has("telemetry");
  next.widgets.pedals = next.widgets.pedals && has("inputs");
  next.widgets.steering = next.widgets.steering && has("inputs");
  next.widgets.input_history = next.widgets.input_history && has("inputs");
  next.widgets.lap_info = next.widgets.lap_info && has("lap_timing");
  next.widgets.lap_timing = next.widgets.lap_timing && has("lap_timing");
  next.widgets.delta_timing = next.widgets.delta_timing && has("timing");
  next.widgets.sectors = next.widgets.sectors && has("sectors");
  next.widgets.mini_sector_widget = next.widgets.mini_sector_widget && has("mini_sectors");
  next.widgets.coaching = next.widgets.coaching && has("coaching");
  next.widgets.performance_monitor = next.widgets.performance_monitor && has("performance");
  for (const [id, widget] of Object.entries(next.extra_widgets)) {
    widget.enabled = widget.enabled && has(id);
  }
  for (const [id, layout] of Object.entries(layer.layout_overrides)) {
    if (id in next.layout) {
      (next.layout as unknown as Record<string, WidgetLayout>)[id] = layout;
    } else if (next.extra_widgets[id]) {
      next.extra_widgets[id].layout = layout;
    }
  }
  return next;
}

function activeLegacyWidgetEnabled(config: OverlayConfig, overlayId: string, key: keyof WidgetConfig): boolean {
  const surface = legacySurfaceByWidgetKey[key];
  if (!surface || !config.widgets[key]) {
    return config.widgets[key];
  }
  const overlay = config.overlays.find((item) => item.id === overlayId);
  return overlay ? overlay.widgets.includes(surface) : config.widgets[key];
}

function catalogSurfaceId(id: string): string {
  const key = legacyWidgetById[id];
  return key ? legacySurfaceByWidgetKey[key] ?? id : id;
}

function nextOverlayId(overlays: OverlayLayerConfig[]): string {
  const ids = new Set(overlays.map((overlay) => overlay.id));
  let ordinal = 1;
  while (ids.has(`overlay-${ordinal}`)) {
    ordinal += 1;
  }
  return `overlay-${ordinal}`;
}

function overlayOrdinal(id: string): number {
  const ordinal = Number(id.replace("overlay-", ""));
  return Number.isSafeInteger(ordinal) && ordinal > 0 ? ordinal : 1;
}

function layoutForSelection(config: OverlayConfig, selection: LayoutSelection): WidgetLayout {
  if (selection.startsWith("extra:")) {
    return config.extra_widgets[selection.slice("extra:".length)]?.layout ?? config.layout.telemetry;
  }
  return config.layout[selection as LayoutWidgetKey];
}

function updateLayoutSelection(config: OverlayConfig, selection: LayoutSelection, layout: WidgetLayout): OverlayConfig {
  if (selection.startsWith("extra:")) {
    const id = selection.slice("extra:".length);
    const widget = config.extra_widgets[id];
    return widget ? { ...config, extra_widgets: { ...config.extra_widgets, [id]: { ...widget, layout } } } : config;
  }
  return { ...config, layout: { ...config.layout, [selection]: layout } };
}

function updateOverlayLayoutSelection(config: OverlayConfig, overlayId: string, selection: LayoutSelection, layout: WidgetLayout): OverlayConfig {
  const next = updateLayoutSelection(config, selection, layout);
  const key = selection.startsWith("extra:") ? selection.slice("extra:".length) : selection;
  return {
    ...next,
    overlays: next.overlays.map((overlay) => overlay.id === overlayId
      ? { ...overlay, layout_overrides: { ...overlay.layout_overrides, [key]: layout } }
      : overlay),
  };
}

function layoutOverridesFromArrangedProfile(
  layout: LayoutConfig,
  widgets: Record<string, WidgetInstanceConfig>,
): Record<string, WidgetLayout> {
  return {
    ...Object.fromEntries(layoutLabels.map(([key]) => [key, structuredClone(layout[key])])),
    ...Object.fromEntries(Object.entries(widgets)
      .filter(([, widget]) => widget.enabled)
      .map(([id, widget]) => [id, structuredClone(widget.layout)])),
  };
}

function snapLayout(config: OverlayConfig, widget: LayoutSelection, layout: WidgetLayout): WidgetLayout {
  let next = { ...layout };
  if (config.layout.snap_to_grid) {
    next = {
      ...next,
      x: snapNumber(next.x, config.layout.grid_size),
      y: snapNumber(next.y, config.layout.grid_size),
      width: snapNumber(next.width, config.layout.grid_size),
      height: snapNumber(next.height, config.layout.grid_size),
    };
  }

  if (config.layout.snap_to_edges) {
    next = snapToEdges(config, next);
  }
  if (config.layout.snap_to_widgets) {
    next = snapToWidgets(config, widget, next);
  }
  return constrainLayoutToWindow(config.window, next);
}

function constrainLayoutToWindow(window: WindowConfig, layout: WidgetLayout): WidgetLayout {
  const scale = clamp(layout.scale, 0.5, 2);
  const width = clamp(layout.width, 48, Math.floor(window.width / scale));
  const height = clamp(layout.height, 20, Math.floor(window.height / scale));
  const visualWidth = scaledDimension(width, scale);
  const visualHeight = scaledDimension(height, scale);
  return {
    ...layout,
    width,
    height,
    x: clamp(layout.x, 0, Math.max(0, window.width - visualWidth)),
    y: clamp(layout.y, 0, Math.max(0, window.height - visualHeight)),
  };
}

function scaledDimension(value: number, scale: number) {
  return Math.round(value * clamp(scale, 0.5, 2));
}

function snapNumber(value: number, gridSize: number) {
  const size = [5, 10, 20].includes(gridSize) ? gridSize : 10;
  return Math.round(value / size) * size;
}

function snapToEdges(config: OverlayConfig, layout: WidgetLayout): WidgetLayout {
  const next = { ...layout };
  const distance = config.layout.snap_distance;
  const width = scaledDimension(next.width, next.scale);
  const height = scaledDimension(next.height, next.scale);
  if (Math.abs(next.x) <= distance) {
    next.x = 0;
  }
  if (Math.abs(next.y) <= distance) {
    next.y = 0;
  }
  if (Math.abs(config.window.width - (next.x + width)) <= distance) {
    next.x = config.window.width - width;
  }
  if (Math.abs(config.window.height - (next.y + height)) <= distance) {
    next.y = config.window.height - height;
  }
  return next;
}

function snapToWidgets(config: OverlayConfig, widget: LayoutSelection, layout: WidgetLayout): WidgetLayout {
  let next = { ...layout };
  const distance = config.layout.snap_distance;
  for (const { key, layout: other } of layoutEntries(config)) {
    if (key === widget) {
      continue;
    }
    const nextWidth = scaledDimension(next.width, next.scale);
    const nextHeight = scaledDimension(next.height, next.scale);
    const otherWidth = scaledDimension(other.width, other.scale);
    const otherHeight = scaledDimension(other.height, other.scale);
    const otherRight = other.x + otherWidth;
    const otherBottom = other.y + otherHeight;
    const xTargets = [
      other.x,
      otherRight,
      other.x - nextWidth,
      otherRight - nextWidth,
    ];
    const yTargets = [
      other.y,
      otherBottom,
      other.y - nextHeight,
      otherBottom - nextHeight,
    ];
    const nearest = (value: number, targets: number[]) => targets.reduce((best, target) =>
      Math.abs(target - value) < Math.abs(best - value) ? target : best,
    targets[0]);
    const xTarget = nearest(next.x, xTargets);
    const yTarget = nearest(next.y, yTargets);
    if (Math.abs(xTarget - next.x) <= distance) {
      next.x = xTarget;
    }
    if (Math.abs(yTarget - next.y) <= distance) {
      next.y = yTarget;
    }
  }
  return next;
}

function Section({ icon, title, children }: SectionProps) {
  return (
    <section className="panel">
      <h2>
        {icon}
        {title}
      </h2>
      <div className="fields">{children}</div>
    </section>
  );
}

function WidgetLayoutFields(props: {
  layout: WidgetLayout;
  defaultLayout: WidgetLayout;
  windowWidth: number;
  windowHeight: number;
  onChange: <K extends keyof WidgetLayout>(key: K, value: WidgetLayout[K]) => void;
  onReplace: (layout: WidgetLayout) => void;
}) {
  const [copiedGeometry, setCopiedGeometry] = useState(false);
  return (
    <>
      <div className="buttonRow geometryActions">
        <button className="secondaryButton" onClick={() => {
          void navigator.clipboard?.writeText(`x=${props.layout.x}, y=${props.layout.y}, width=${props.layout.width}, height=${props.layout.height}, scale=${props.layout.scale}`);
          setCopiedGeometry(true);
          window.setTimeout(() => setCopiedGeometry(false), 1400);
        }}>{copiedGeometry ? "Copied" : "Copy geometry"}</button>
        <button className="secondaryButton" onClick={() => props.onChange("x", Math.max(0, Math.round((props.windowWidth - scaledDimension(props.layout.width, props.layout.scale)) / 2)))}>Center horizontal</button>
        <button className="secondaryButton" onClick={() => props.onChange("y", Math.max(0, Math.round((props.windowHeight - scaledDimension(props.layout.height, props.layout.scale)) / 2)))}>Center vertical</button>
        <button className="secondaryButton" onClick={() => props.onChange("x", 0)}>Align left</button>
        <button className="secondaryButton" onClick={() => props.onChange("x", Math.max(0, props.windowWidth - scaledDimension(props.layout.width, props.layout.scale)))}>Align right</button>
        <button className="secondaryButton" onClick={() => props.onChange("y", 0)}>Align top</button>
        <button className="secondaryButton" onClick={() => props.onChange("y", Math.max(0, props.windowHeight - scaledDimension(props.layout.height, props.layout.scale)))}>Align bottom</button>
        <button className="secondaryButton" onClick={() => {
          props.onReplace({ ...props.defaultLayout });
        }}>Reset widget</button>
        <button className="secondaryButton" onClick={() => {
          props.onReplace({ ...props.layout, width: 320, height: 80 });
        }}>Reset size</button>
      </div>
      <NumberField label="Widget width" value={props.layout.width} onChange={(value) => props.onChange("width", value)} />
      <NumberField label="Widget height" value={props.layout.height} onChange={(value) => props.onChange("height", value)} />
      <RangeField label="Widget scale" min={0.5} max={2} step={0.05} value={props.layout.scale} onChange={(value) => props.onChange("scale", value)} />
      <RangeField label="Widget opacity" min={0.1} max={1} step={0.05} value={props.layout.opacity} onChange={(value) => props.onChange("opacity", value)} />
      <NumberField label="Layer order" value={props.layout.z_index} onChange={(value) => props.onChange("z_index", value)} />
      <label className="toggle full">
        <input type="checkbox" checked={props.layout.locked} onChange={(event) => props.onChange("locked", event.target.checked)} />
        <Lock size={16} />
        <span>Lock this widget</span>
      </label>
    </>
  );
}

function NumberField(props: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <input type="number" value={props.value} onChange={(event) => props.onChange(Number(event.target.value))} />
    </label>
  );
}

function RangeField(props: { label: string; min: number; max: number; step: number; value: number; onChange: (value: number) => void }) {
  return (
    <label className="field range">
      <span>{props.label}</span>
      <input type="range" min={props.min} max={props.max} step={props.step} value={props.value} onChange={(event) => props.onChange(Number(event.target.value))} />
      <output>{props.value}</output>
    </label>
  );
}

function Segmented(props: { value: string; options: string[][]; onChange: (value: string) => void }) {
  return (
    <div className="segmented">
      {props.options.map(([value, label]) => (
        <button key={value} className={props.value === value ? "selected" : ""} onClick={() => props.onChange(value)}>
          {label}
        </button>
      ))}
    </div>
  );
}

function setPerformanceWindow<K extends keyof Pick<WindowConfig, "refresh_hz" | "sample_ms" | "history_samples">>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  key: K,
  value: WindowConfig[K],
) {
  const normalizedValue = normalizeWindowField(key, value);
  setConfig({
    ...config,
    performance: { mode: "custom" },
    window: { ...config.window, [key]: normalizedValue },
    overlays: config.overlays.map((overlay) => ({
      ...overlay,
      window: { ...overlay.window, [key]: normalizedValue },
    })),
  });
}

function setOverlayWindow<K extends keyof WindowConfig>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  overlayId: string,
  key: K,
  value: WindowConfig[K],
) {
  const normalizedValue = normalizeWindowField(key, value);
  const next = { ...config, window: { ...config.window, [key]: normalizedValue } };
  setConfig({
    ...next,
    overlays: next.overlays.map((overlay) => overlay.id === overlayId
      ? { ...overlay, window: { ...overlay.window, [key]: normalizedValue } }
      : overlay),
  });
}

function normalizeWindowField<K extends keyof WindowConfig>(key: K, value: WindowConfig[K]): WindowConfig[K] {
  if (key === "x" || key === "y") {
    return Math.round(Number(value)) as WindowConfig[K];
  }
  if (key === "width" || key === "height") {
    return Math.max(48, Math.min(7680, Math.round(Number(value)))) as WindowConfig[K];
  }
  if (key === "refresh_hz") {
    return Math.max(15, Math.min(240, Math.round(Number(value)))) as WindowConfig[K];
  }
  if (key === "sample_ms") {
    return Math.max(5, Math.min(1000, Math.round(Number(value)))) as WindowConfig[K];
  }
  if (key === "history_samples") {
    return Math.max(16, Math.min(10000, Math.round(Number(value)))) as WindowConfig[K];
  }
  return value;
}

function setOverlayPosition(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  overlayId: string,
  x: number,
  y: number,
) {
  const next = { ...config, window: { ...config.window, x, y } };
  setConfig({
    ...next,
    overlays: next.overlays.map((overlay) => overlay.id === overlayId
      ? { ...overlay, window: { ...overlay.window, x, y } }
      : overlay),
  });
}

function setStyle<K extends keyof StyleConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: StyleConfig[K]) {
  setConfig({ ...config, style: { ...config.style, [key]: value } });
}

function applyTheme(config: OverlayConfig, theme: string): OverlayConfig {
  const palettes: Record<string, Partial<StyleConfig>> = {
    hashoverlay_default: {
      background: "#101318",
      border: "#f2bc57",
      primary_text: "#f8fafc",
      secondary_text: "#b8c0cc",
      throttle: "#36d36a",
      brake: "#ff4f42",
      clutch: "#31c8d8",
      steering: "#f8fafc",
      delta_gain: "#36d36a",
      delta_loss: "#ff4f42",
      delta_neutral: "#f2bc57",
      reference: "#8ea0b8",
      rpm: "#f2bc57",
      coaching_warning: "#f2bc57",
      coaching_positive: "#36d36a",
    },
    minimal_dark: {
      background: "#07090d",
      border: "#3a4250",
      primary_text: "#f5f7fb",
      secondary_text: "#8f9bad",
      throttle: "#2fd35f",
      brake: "#ff5148",
      clutch: "#28bfd2",
      steering: "#dfe6f0",
      delta_gain: "#2fd35f",
      delta_loss: "#ff5148",
      delta_neutral: "#b7c0ce",
      reference: "#737f90",
      rpm: "#f0c24d",
      coaching_warning: "#f0c24d",
      coaching_positive: "#2fd35f",
    },
    transparent: {
      background: "#000000",
      border: "#5d7188",
      primary_text: "#ffffff",
      secondary_text: "#c8d3e0",
      throttle: "#34e676",
      brake: "#ff5148",
      clutch: "#2ed7e5",
      steering: "#ffffff",
      delta_gain: "#34e676",
      delta_loss: "#ff5148",
      delta_neutral: "#ffd166",
      reference: "#9aa8ba",
      rpm: "#ffd166",
      coaching_warning: "#ffd166",
      coaching_positive: "#34e676",
    },
    high_contrast: {
      background: "#000000",
      border: "#ffffff",
      primary_text: "#ffffff",
      secondary_text: "#e5e7eb",
      throttle: "#00ff66",
      brake: "#ff2b2b",
      clutch: "#00e5ff",
      steering: "#ffffff",
      delta_gain: "#00ff66",
      delta_loss: "#ff2b2b",
      delta_neutral: "#ffe45e",
      reference: "#d1d5db",
      rpm: "#ffe45e",
      coaching_warning: "#ffe45e",
      coaching_positive: "#00ff66",
    },
  };
  return {
    ...config,
    style: {
      ...config.style,
      ...palettes[theme],
      theme,
      opacity: theme === "transparent" ? Math.min(config.style.opacity, 190) : config.style.opacity,
    },
  };
}

function setTiming<K extends keyof TimingConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: TimingConfig[K]) {
  setConfig({ ...config, timing: { ...config.timing, [key]: value } });
}

function setWidget(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  activeOverlayId: string,
  key: keyof WidgetConfig,
  value: boolean,
) {
  const widgets = { ...config.widgets, [key]: value };
  const surface = legacySurfaceByWidgetKey[key];
  const overlays = surface
    ? config.overlays.map((overlay) => overlay.id === activeOverlayId
      ? {
          ...overlay,
          widgets: value
            ? [...new Set([...overlay.widgets, surface])]
            : overlay.widgets.filter((widget) => widget !== surface || legacySurfaceEnabled(widgets, surface)),
        }
      : overlay)
    : config.overlays;
  setConfig({
    ...config,
    widgets: surface && !value && overlayIdUsedByOtherLayer(overlays, activeOverlayId, surface)
      ? { ...widgets, [key]: true }
      : widgets,
    overlays,
  });
}

function enabledLegacySurfaceIds(widgets: WidgetConfig): LayoutWidgetKey[] {
  return layoutLabels
    .map(([key]) => key)
    .filter((surface) => legacySurfaceEnabled(widgets, surface));
}

function legacySurfaceEnabled(widgets: WidgetConfig, surface: LayoutWidgetKey): boolean {
  return Object.entries(legacySurfaceByWidgetKey).some(([key, value]) => (
    value === surface && widgets[key as keyof WidgetConfig]
  ));
}

function overlayIdUsedByOtherLayer(overlays: OverlayLayerConfig[], activeOverlayId: string, id: string): boolean {
  return overlays.some((overlay) => overlay.id !== activeOverlayId && overlay.widgets.includes(id));
}

function syncExtraWidgetsWithOverlayMembership(config: OverlayConfig): OverlayConfig {
  const usedIds = new Set(config.overlays.flatMap((overlay) => overlay.widgets));
  const widgets = { ...config.widgets };
  for (const id of usedIds) {
    const key = legacyWidgetById[id];
    if (key) {
      widgets[key] = true;
    }
  }
  return {
    ...config,
    widgets,
    extra_widgets: Object.fromEntries(Object.entries(config.extra_widgets).map(([id, widget]) => [
      id,
      usedIds.has(id) ? { ...widget, enabled: true } : widget,
    ])),
  };
}

function setUnits<K extends keyof UnitsConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: UnitsConfig[K]) {
  setConfig({ ...config, units: { ...config.units, [key]: value } });
}

function setCoaching<K extends keyof CoachingConfig>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  key: K,
  value: CoachingConfig[K],
) {
  setConfig({ ...config, coaching: { ...config.coaching, [key]: value } });
}

function HotkeyField(props: { label: string; value: string; conflict: boolean; onChange: (value: string) => void }) {
  return (
    <label className={`field ${props.conflict ? "invalid" : ""}`}>
      <span>{props.label}</span>
      <button
        className="hotkeyButton"
        onKeyDown={(event) => {
          event.preventDefault();
          const parts = [];
          if (event.ctrlKey) {
            parts.push("Ctrl");
          }
          if (event.shiftKey) {
            parts.push("Shift");
          }
          if (event.altKey) {
            parts.push("Alt");
          }
          const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
          if (!["Control", "Shift", "Alt"].includes(key)) {
            parts.push(key.toUpperCase());
            props.onChange(parts.join("+"));
          }
        }}
      >
        {props.value || "Press a key"}
      </button>
      {props.conflict && <small>Conflict</small>}
    </label>
  );
}

function WidgetStyleFields(props: {
  style: WidgetStyleConfig;
  onChange: <K extends keyof WidgetStyleConfig>(key: K, value: WidgetStyleConfig[K]) => void;
}) {
  const colorFields: Array<[keyof Pick<WidgetStyleConfig, "background_color" | "border_color" | "primary_color" | "secondary_color" | "accent_color">, string]> = [
    ["background_color", "Widget background"],
    ["border_color", "Widget border"],
    ["primary_color", "Value color"],
    ["secondary_color", "Title color"],
    ["accent_color", "Accent color"],
  ];
  return (
    <>
      <label className="toggle full">
        <input type="checkbox" checked={props.style.inherit_theme} onChange={(event) => props.onChange("inherit_theme", event.target.checked)} />
        <Paintbrush size={16} />
        <span>Use global appearance</span>
      </label>
      {!props.style.inherit_theme && <button className="secondaryButton" onClick={() => props.onChange("inherit_theme", true)}>
        <RotateCcw size={16} />
        Reset widget style to theme
      </button>}
      {!props.style.inherit_theme && <>
        <label className="toggle full">
          <input type="checkbox" checked={props.style.show_background} onChange={(event) => props.onChange("show_background", event.target.checked)} />
          <span>Show widget background</span>
        </label>
        <label className="toggle full">
          <input type="checkbox" checked={props.style.show_border} onChange={(event) => props.onChange("show_border", event.target.checked)} />
          <span>Show widget border</span>
        </label>
        <NumberField label="Border width" value={props.style.border_width} onChange={(value) => props.onChange("border_width", value)} />
        <NumberField label="Border radius" value={props.style.border_radius} onChange={(value) => props.onChange("border_radius", value)} />
        <NumberField label="Inner padding" value={props.style.padding} onChange={(value) => props.onChange("padding", value)} />
        <div className="swatches">
          {colorFields.map(([key, label]) => (
            <label className="swatch" key={key}>
              <span>{label}</span>
              <input type="color" value={props.style[key]} onChange={(event) => props.onChange(key, event.target.value)} />
            </label>
          ))}
        </div>
      </>}
      <label className="toggle full">
        <input type="checkbox" checked={props.style.show_title} onChange={(event) => props.onChange("show_title", event.target.checked)} />
        <span>Show widget title</span>
      </label>
      {props.style.show_title && <TextInput label="Widget title" value={props.style.title_text} onChange={(value) => props.onChange("title_text", value)} />}
    </>
  );
}

function WidgetOptionsFields(props: {
  id: string;
  options: WidgetOptions;
  onChange: <K extends keyof WidgetOptions>(key: K, value: WidgetOptions[K]) => void;
}) {
  if (!["relative", "standings", "fuel", "tyres", "brakes", "rpm"].includes(props.id)) return null;
  return <>
    {props.id === "relative" && <>
      <NumberField label="Cars ahead" value={props.options.cars_ahead} onChange={(value) => props.onChange("cars_ahead", value)} />
      <NumberField label="Cars behind" value={props.options.cars_behind} onChange={(value) => props.onChange("cars_behind", value)} />
    </>}
    {props.id === "standings" && <>
      <NumberField label="Rows" value={props.options.rows} onChange={(value) => props.onChange("rows", value)} />
      <label className="toggle full"><input type="checkbox" checked={props.options.same_class_only} onChange={(event) => props.onChange("same_class_only", event.target.checked)} /><span>Same class only</span></label>
    </>}
    {(props.id === "relative" || props.id === "standings") && <>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_driver} onChange={(event) => props.onChange("show_driver", event.target.checked)} /><span>Show driver</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_car} onChange={(event) => props.onChange("show_car", event.target.checked)} /><span>Show car</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_position} onChange={(event) => props.onChange("show_position", event.target.checked)} /><span>Show position</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_laps} onChange={(event) => props.onChange("show_laps", event.target.checked)} /><span>Show laps</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_class} onChange={(event) => props.onChange("show_class", event.target.checked)} /><span>Show class</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_gap} onChange={(event) => props.onChange("show_gap", event.target.checked)} /><span>Show gap</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_pit} onChange={(event) => props.onChange("show_pit", event.target.checked)} /><span>Show pit state</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_last_lap} onChange={(event) => props.onChange("show_last_lap", event.target.checked)} /><span>Show last lap</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_best_lap} onChange={(event) => props.onChange("show_best_lap", event.target.checked)} /><span>Show best lap</span></label>
    </>}
    {props.id === "fuel" && <>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_average} onChange={(event) => props.onChange("show_average", event.target.checked)} /><span>Show average usage</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_last_lap} onChange={(event) => props.onChange("show_last_lap", event.target.checked)} /><span>Show last lap usage</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_estimated_laps} onChange={(event) => props.onChange("show_estimated_laps", event.target.checked)} /><span>Show estimated laps</span></label>
    </>}
    {props.id === "tyres" && <>
      <Segmented value={props.options.tyre_temperature_mode} options={tyreTemperatureModes} onChange={(value) => props.onChange("tyre_temperature_mode", value)} />
      <label className="toggle full"><input type="checkbox" checked={props.options.show_wear} onChange={(event) => props.onChange("show_wear", event.target.checked)} /><span>Show tyre wear</span></label>
    </>}
    {props.id === "brakes" && <>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_brake_temperature} onChange={(event) => props.onChange("show_brake_temperature", event.target.checked)} /><span>Show brake temperature</span></label>
      <label className="toggle full"><input type="checkbox" checked={props.options.show_brake_pressure} onChange={(event) => props.onChange("show_brake_pressure", event.target.checked)} /><span>Show brake pressure</span></label>
    </>}
    {props.id === "rpm" && <>
      <NumberField label="Shift start %" value={props.options.shift_start_percent} onChange={(value) => props.onChange("shift_start_percent", value)} />
      <NumberField label="Shift warning %" value={props.options.shift_warning_percent} onChange={(value) => props.onChange("shift_warning_percent", value)} />
      <NumberField label="Limiter %" value={props.options.limiter_percent} onChange={(value) => props.onChange("limiter_percent", value)} />
      <NumberField label="Shift light segments" value={props.options.shift_segments} onChange={(value) => props.onChange("shift_segments", value)} />
    </>}
  </>;
}

function hasHotkeyConflict(hotkeys: HotkeyConfig, key: keyof HotkeyConfig) {
  const value = normalizeHotkey(hotkeys[key]);
  if (!value) {
    return false;
  }
  return Object.entries(hotkeys).some(([otherKey, otherValue]) => otherKey !== key && normalizeHotkey(otherValue) === value);
}

function normalizeHotkey(value: string) {
  return value.trim().toUpperCase().replace(/\s+/g, "");
}

function TextInput(props: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <input value={props.value} onChange={(event) => props.onChange(event.target.value)} />
    </label>
  );
}

function setLayoutFlag<K extends keyof Omit<LayoutConfig, "telemetry" | "inputs" | "lap_timing" | "timing" | "sectors" | "mini_sectors" | "coaching" | "performance">>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  key: K,
  value: LayoutConfig[K],
) {
  setConfig({ ...config, layout: { ...config.layout, [key]: value } });
}

function setOverlayLayoutSelection<K extends keyof WidgetLayout>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  overlayId: string,
  selection: LayoutSelection,
  key: K,
  value: WidgetLayout[K],
) {
  const view = overlayPreviewConfig(config, overlayId);
  const layout = layoutForSelection(view, selection);
  const next = normalizeWidgetLayoutForWindow(view.window, { ...layout, [key]: value });
  setConfig(updateOverlayLayoutSelection(config, overlayId, selection, next));
}

function normalizeWidgetLayoutForWindow(window: WindowConfig, layout: WidgetLayout): WidgetLayout {
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

function setExtraWidgetStyle<K extends keyof WidgetStyleConfig>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  id: string,
  key: K,
  value: WidgetStyleConfig[K],
) {
  const widget = config.extra_widgets[id];
  if (!widget) {
    return;
  }
  setConfig({ ...config, extra_widgets: { ...config.extra_widgets, [id]: { ...widget, style: { ...widget.style, [key]: value } } } });
}

function setExtraWidgetOption<K extends keyof WidgetOptions>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  id: string,
  key: K,
  value: WidgetOptions[K],
) {
  const widget = config.extra_widgets[id];
  if (!widget) return;
  setConfig({ ...config, extra_widgets: { ...config.extra_widgets, [id]: { ...widget, options: { ...widget.options, [key]: value } } } });
}

function setHotkey(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: keyof HotkeyConfig, value: string) {
  setConfig({ ...config, hotkeys: { ...config.hotkeys, [key]: value } });
}

function setPerformance(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, mode: string) {
  setConfig(applyPerformanceMode(config, mode));
}

function applyPerformanceMode(config: OverlayConfig, mode: string): OverlayConfig {
  const values = performanceWindowValues[mode];
  const applyToWindow = (window: WindowConfig) => values ? { ...window, ...values } : window;
  return {
    ...config,
    performance: { mode },
    window: applyToWindow(config.window),
    overlays: config.overlays.map((overlay) => ({ ...overlay, window: applyToWindow(overlay.window) })),
  };
}

function addCustomPreset(config: OverlayConfig): OverlayConfig {
  const nextPreset: CustomPresetConfig = {
    name: `Custom ${config.presets.custom.length + 1}`,
    profile: currentProfile(config),
  };
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: [...config.presets.custom, nextPreset].slice(0, 32),
    },
  };
}

function renameCustomPreset(config: OverlayConfig, index: number, name: string): OverlayConfig {
  return updateCustomPreset(config, index, (preset) => ({ ...preset, name }));
}

function saveCurrentIntoCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  return updateCustomPreset(config, index, (preset) => ({ ...preset, profile: currentProfile(config) }));
}

function deleteCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: config.presets.custom.filter((_, presetIndex) => presetIndex !== index),
    },
  };
}

function applyCustomPreset(config: OverlayConfig, index: number, overlayId?: string): OverlayConfig {
  const preset = config.presets.custom[index];
  if (!preset) {
    return config;
  }
  return applyProfile(config, preset.profile, overlayId);
}

function updateCustomPreset(
  config: OverlayConfig,
  index: number,
  update: (preset: CustomPresetConfig) => CustomPresetConfig,
): OverlayConfig {
  return {
    ...config,
    presets: {
      ...config.presets,
      custom: config.presets.custom.map((preset, presetIndex) => (presetIndex === index ? update(preset) : preset)),
    },
  };
}

function pickWidgets(profile: PresetProfileConfig): WidgetConfig {
  return Object.fromEntries(widgetLabels.map(([key]) => [key, profile[key]])) as WidgetConfig;
}

function currentProfile(config: OverlayConfig): PresetProfileConfig {
  return {
    performance_mode: config.performance.mode,
    reference_mode: config.timing.reference_mode,
    mini_sectors: config.timing.mini_sectors,
    style: structuredClone(config.style),
    units: structuredClone(config.units),
    coaching_config: structuredClone(config.coaching),
    layout: structuredClone(config.layout),
    extra_widgets: structuredClone(config.extra_widgets),
    ...config.widgets,
  };
}

function profilesMatch(left: PresetProfileConfig, right: PresetProfileConfig): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function applyProfile(config: OverlayConfig, profile: PresetProfileConfig, overlayId?: string): OverlayConfig {
  const arranged = arrangeProfileLayout(profile);
  const next = applyPerformanceMode({
    ...config,
    timing: {
      ...config.timing,
      reference_mode: profile.reference_mode,
      mini_sectors: profile.mini_sectors,
    },
    widgets: pickWidgets(profile),
    style: structuredClone(profile.style),
    units: structuredClone(profile.units),
    coaching: structuredClone(profile.coaching_config),
    layout: arranged.layout,
    extra_widgets: arranged.widgets,
    window: { ...config.window, height: arranged.height },
  }, profile.performance_mode);
  if (!overlayId) {
    return next;
  }

  const enabledWidgets = new Set<string>();
  for (const id of enabledLegacySurfaceIds(next.widgets)) {
    enabledWidgets.add(id);
  }
  for (const [id, widget] of Object.entries(next.extra_widgets)) {
    if (widget.enabled) {
      enabledWidgets.add(id);
    }
  }
  const overlays = next.overlays.map((overlay) => overlay.id === overlayId
      ? {
          ...overlay,
          window: { ...overlay.window, height: arranged.height },
          widgets: [...enabledWidgets],
          layout_overrides: layoutOverridesFromArrangedProfile(arranged.layout, arranged.widgets),
        }
      : overlay);
  return syncExtraWidgetsWithOverlayMembership({ ...next, overlays });
}

function arrangeProfileLayout(profile: PresetProfileConfig) {
  const layout = structuredClone(profile.layout);
  const widgets = structuredClone(profile.extra_widgets);
  const left = 14;
  const width = 392;
  const gap = 10;
  let y = 10;
  let z = 10;

  const place = (key: LayoutWidgetKey, x: number, nextY: number, nextWidth: number, height: number) => {
    layout[key] = {
      ...layout[key],
      x,
      y: nextY,
      width: nextWidth,
      height,
      z_index: z,
    };
    z += 10;
  };
  const full = (key: LayoutWidgetKey, height: number) => {
    place(key, left, y, width, height);
    y += height + gap;
  };
  const row = (items: Array<[LayoutWidgetKey, number]>) => {
    if (items.length === 0) {
      return;
    }
    if (items.length === 1) {
      full(items[0][0], items[0][1]);
      return;
    }
    const columnWidth = Math.floor((width - gap) / 2);
    const rowHeight = Math.max(...items.map(([, height]) => height));
    items.forEach(([key, height], index) => {
      place(key, left + index * (columnWidth + gap), y, columnWidth, height);
    });
    y += rowHeight + gap;
  };

  if (legacySurfaceEnabled(profile, "telemetry")) {
    full("telemetry", 52);
  }
  row([
    ...(legacySurfaceEnabled(profile, "inputs") ? [["inputs", 96] as [LayoutWidgetKey, number]] : []),
    ...(legacySurfaceEnabled(profile, "lap_timing") ? [["lap_timing", 44] as [LayoutWidgetKey, number]] : []),
  ]);
  row([
    ...(legacySurfaceEnabled(profile, "timing") ? [["timing", 54] as [LayoutWidgetKey, number]] : []),
    ...(legacySurfaceEnabled(profile, "sectors") ? [["sectors", 44] as [LayoutWidgetKey, number]] : []),
  ]);
  row([
    ...(legacySurfaceEnabled(profile, "mini_sectors") ? [["mini_sectors", 44] as [LayoutWidgetKey, number]] : []),
    ...(legacySurfaceEnabled(profile, "performance") ? [["performance", 28] as [LayoutWidgetKey, number]] : []),
  ]);
  if (legacySurfaceEnabled(profile, "coaching")) {
    full("coaching", 58);
  }

  const extra = arrangeExtraWidgets(widgets, y, z);
  return { layout, widgets: extra.widgets, height: Math.max(220, extra.height) };
}

function arrangeExtraWidgets(widgets: Record<string, WidgetInstanceConfig>, startY = 326, startZ = 100) {
  const next = structuredClone(widgets);
  const enabled = Object.entries(next)
    .filter(([, widget]) => widget.enabled)
    .sort(([left], [right]) => Number(right === "standings") - Number(left === "standings"));
  const columnWidth = 196;
  const rowHeight = 52;
  const gap = 10;
  const left = 14;
  const top = startY;
  let slot = 0;

  for (const [id, widget] of enabled) {
    const isStandings = id === "standings";
    if (isStandings) {
      widget.layout = { ...widget.layout, x: left, y: top, width: 392, height: 120, z_index: startZ + slot };
      slot += 4;
      continue;
    }
    const column = slot % 2;
    const row = Math.floor(slot / 2);
    widget.layout = {
      ...widget.layout,
      x: left + column * (columnWidth + gap),
      y: top + row * (rowHeight + gap),
      width: columnWidth,
      height: rowHeight,
      z_index: startZ + slot,
    };
    slot += 1;
  }

  const rows = Math.max(1, Math.ceil(slot / 2));
  return { widgets: next, height: Math.max(450, top + rows * rowHeight + (rows - 1) * gap + 14) };
}

function titleCase(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

export default App;
