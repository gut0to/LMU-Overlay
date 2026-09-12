import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
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
  Timer,
  Trash2,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type WindowConfig = {
  x: number;
  y: number;
  width: number;
  height: number;
  refresh_hz: number;
  sample_ms: number;
  history_samples: number;
};

type StyleConfig = {
  opacity: number;
  scale: number;
  line_thickness: number;
  background: string;
  border: string;
  primary_text: string;
  secondary_text: string;
  throttle: string;
  brake: string;
  clutch: string;
  steering: string;
  delta_gain: string;
  delta_loss: string;
  delta_neutral: string;
  reference: string;
  rpm: string;
  coaching_warning: string;
  coaching_positive: string;
  font_family: string;
  font_size: number;
  font_weight: number;
  large_number_size: number;
  theme: string;
};

type WidgetConfig = {
  title: boolean;
  speed_gear_rpm: boolean;
  pedals: boolean;
  steering: boolean;
  lap_info: boolean;
  lap_timing: boolean;
  sectors: boolean;
  mini_sector_widget: boolean;
  input_history: boolean;
  delta_timing: boolean;
  ghost_inputs: boolean;
  coaching: boolean;
  performance_monitor: boolean;
};

type WidgetDefinition = {
  id: string;
  name: string;
  category: string;
  description: string;
  data_requirement: string;
};

type WidgetLayout = {
  x: number;
  y: number;
  width: number;
  height: number;
  locked: boolean;
  scale: number;
  opacity: number;
  z_index: number;
};

type WidgetStyleConfig = {
  inherit_theme: boolean;
  show_background: boolean;
  background_color: string;
  show_border: boolean;
  border_color: string;
  border_width: number;
  padding: number;
  font_scale: number;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  show_title: boolean;
  title_text: string;
};

type WidgetInstanceConfig = {
  enabled: boolean;
  layout: WidgetLayout;
  style: WidgetStyleConfig;
};

type LayoutConfig = {
  lock_all: boolean;
  snap_to_edges: boolean;
  snap_to_grid: boolean;
  snap_to_widgets: boolean;
  grid_size: number;
  snap_distance: number;
  telemetry: WidgetLayout;
  inputs: WidgetLayout;
  lap_timing: WidgetLayout;
  timing: WidgetLayout;
  sectors: WidgetLayout;
  mini_sectors: WidgetLayout;
  coaching: WidgetLayout;
  performance: WidgetLayout;
};

type LayoutWidgetKey = "telemetry" | "inputs" | "lap_timing" | "timing" | "sectors" | "mini_sectors" | "coaching" | "performance";
type LayoutSelection = LayoutWidgetKey | `extra:${string}`;

type UnitsConfig = {
  speed: string;
  temperature: string;
  pressure: string;
  fuel: string;
};

type CoachingConfig = {
  mode: string;
  brake_timing: boolean;
  throttle_timing: boolean;
  input_match: boolean;
  speed: boolean;
  gear: boolean;
  speed_threshold_kph: number;
  timing_deadband_m: number;
  event_match_tolerance_m: number;
  max_hints: number;
};

type TimingConfig = {
  reference_mode: string;
  mini_sectors: number;
  brake_threshold: number;
  throttle_threshold: number;
};

type HotkeyConfig = {
  toggle_overlay: string;
  edit_mode: string;
  toggle_coaching: string;
  cycle_preset: string;
};

type PerformanceConfig = {
  mode: string;
};

type PresetProfileConfig = {
  performance_mode: string;
  reference_mode: string;
  mini_sectors: number;
  style: StyleConfig;
  units: UnitsConfig;
  coaching_config: CoachingConfig;
  layout: LayoutConfig;
  extra_widgets: Record<string, WidgetInstanceConfig>;
} & WidgetConfig;

type PresetConfig = {
  practice: PresetProfileConfig;
  qualifying: PresetProfileConfig;
  race: PresetProfileConfig;
  endurance: PresetProfileConfig;
  minimal: PresetProfileConfig;
  custom: CustomPresetConfig[];
};

type CustomPresetConfig = {
  name: string;
  profile: PresetProfileConfig;
};

type OverlayConfig = {
  config_version: number;
  window: WindowConfig;
  style: StyleConfig;
  widgets: WidgetConfig;
  extra_widgets: Record<string, WidgetInstanceConfig>;
  layout: LayoutConfig;
  units: UnitsConfig;
  coaching: CoachingConfig;
  timing: TimingConfig;
  hotkeys: HotkeyConfig;
  performance: PerformanceConfig;
  presets: PresetConfig;
};

type LoadResponse = {
  path: string;
  config: OverlayConfig;
};

type SectionProps = {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
};

const presetNames = ["practice", "qualifying", "race", "endurance", "minimal"] as const;
type StandardPresetKey = (typeof presetNames)[number];
const pages = ["Dashboard", "Widgets", "Layout", "Appearance", "Timing", "Coaching", "Performance", "Hotkeys", "Presets", "Advanced"] as const;
type Page = (typeof pages)[number];
const performanceModes = [
  ["eco", "Eco"],
  ["normal", "Normal"],
  ["high_refresh", "High Refresh"],
  ["custom", "Custom"],
];
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

function App() {
  const [config, setConfig] = useState<OverlayConfig | null>(null);
  const [path, setPath] = useState("");
  const [status, setStatus] = useState("Loading config");
  const [saving, setSaving] = useState(false);
  const [selectedLayout, setSelectedLayout] = useState<LayoutSelection>("telemetry");
  const [activePage, setActivePage] = useState<Page>("Dashboard");
  const [defaultConfigState, setDefaultConfigState] = useState<OverlayConfig | null>(null);
  const [configText, setConfigText] = useState("");
  const [widgetCatalog, setWidgetCatalog] = useState<WidgetDefinition[]>([]);
  const [widgetSearch, setWidgetSearch] = useState("");
  const [widgetCategory, setWidgetCategory] = useState("All");

  useEffect(() => {
    void loadConfig();
    void loadDefaultConfig();
    void loadWidgetCatalog();
  }, []);

  async function loadConfig() {
    try {
      const response = await invoke<LoadResponse>("load_config");
      setConfig(response.config);
      setPath(response.path);
      setStatus("Config loaded");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function saveConfig() {
    if (!config) {
      return;
    }
    setSaving(true);
    try {
      const response = await invoke<LoadResponse>("save_config", { config });
      setConfig(response.config);
      setPath(response.path);
      setStatus("Saved");
    } catch (error) {
      setStatus(String(error));
    } finally {
      setSaving(false);
    }
  }

  async function loadDefaultConfig() {
    try {
      const nextDefault = await invoke<OverlayConfig>("default_config");
      setDefaultConfigState(nextDefault);
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
    try {
      const response = await invoke<LoadResponse>("import_config", { text: configText });
      setConfig(response.config);
      setPath(response.path);
      setStatus("Config imported");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function resetConfig() {
    try {
      const response = await invoke<LoadResponse>("reset_config");
      setConfig(response.config);
      setPath(response.path);
      setStatus("Config reset");
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function startOverlay() {
    try {
      await invoke("start_overlay");
      setStatus("Overlay started");
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
    setConfig((current) => current ? { ...current, window: defaultConfigState.window, layout: defaultConfigState.layout } : current);
    setStatus("Layout reset");
  }

  function applyPreset(name: StandardPresetKey) {
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const preset = current.presets[name];
      return applyProfile(current, preset);
    });
  }

  function setCatalogWidget(id: string, enabled: boolean) {
    const legacy = legacyWidgetById[id];
    setConfig((current) => {
      if (!current) {
        return current;
      }
      if (legacy) {
        return { ...current, widgets: { ...current.widgets, [legacy]: enabled } };
      }
      const widget = current.extra_widgets[id];
      return widget
        ? { ...current, extra_widgets: { ...current.extra_widgets, [id]: { ...widget, enabled } } }
        : current;
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
  const availableLayoutEntries = useMemo(() => config ? layoutEntries(config) : [], [config]);

  const activePreset = useMemo(() => {
    if (!config) {
      return null;
    }
    return presetNames.find((name) => {
      const preset = config.presets[name];
      return (
        config.performance.mode === preset.performance_mode &&
        config.timing.reference_mode === preset.reference_mode &&
        config.timing.mini_sectors === preset.mini_sectors &&
        widgetLabels.every(([key]) => config.widgets[key] === preset[key])
      );
    });
  }, [config]);

  if (!config) {
    return (
      <main className="shell loading">
        <Activity size={20} />
        <span>{status}</span>
      </main>
    );
  }

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <h1>HashOverlay Settings</h1>
          <p>{path}</p>
        </div>
        <div className="actions">
          <button className="iconButton" title="Open config folder" onClick={openConfigFolder}>
            <FolderOpen size={18} />
          </button>
          <button className="iconButton" title="Reload config" onClick={loadConfig}>
            <RotateCcw size={18} />
          </button>
          <button className="primaryButton" onClick={startOverlay}>
            <Play size={18} />
            Start overlay
          </button>
          <button className="primaryButton" onClick={saveConfig} disabled={saving}>
            <Save size={18} />
            {saving ? "Saving" : "Save"}
          </button>
        </div>
      </header>

      <section className="presetBar">
        {presetNames.map((name) => (
          <button
            key={name}
            className={activePreset === name ? "selected" : ""}
            onClick={() => applyPreset(name)}
          >
            {titleCase(name)}
          </button>
        ))}
      </section>

      <nav className="tabs">
        {pages.map((page) => (
          <button key={page} className={activePage === page ? "selected" : ""} onClick={() => setActivePage(page)}>
            {page}
          </button>
        ))}
      </nav>

      <div className="grid">
        {showPanel(activePage, "Dashboard", "Layout") && <Section icon={<Activity />} title="Live Preview">
          <OverlayPreview
            config={config}
            selected={selectedLayout}
            onSelect={setSelectedLayout}
            onLayoutChange={(widget, layout) => setConfig(updateLayoutSelection(config, widget, layout))}
          />
        </Section>}

        {showPanel(activePage, "Dashboard", "Layout") && <Section icon={<LayoutGrid />} title="Window">
          <NumberField label="X" value={config.window.x} onChange={(value) => setWindow(config, setConfig, "x", value)} />
          <NumberField label="Y" value={config.window.y} onChange={(value) => setWindow(config, setConfig, "y", value)} />
          <NumberField label="Width" value={config.window.width} onChange={(value) => setWindow(config, setConfig, "width", value)} />
          <NumberField label="Height" value={config.window.height} onChange={(value) => setWindow(config, setConfig, "height", value)} />
          <RangeField label="Scale" min={0.65} max={1.75} step={0.05} value={config.style.scale} onChange={(value) => setStyle(config, setConfig, "scale", value)} />
          <RangeField label="Opacity" min={32} max={255} step={1} value={config.style.opacity} onChange={(value) => setStyle(config, setConfig, "opacity", value)} />
        </Section>}

        {showPanel(activePage, "Dashboard", "Layout") && <Section icon={<Magnet />} title="Widget Layout">
          <div className="segmented">
            {availableLayoutEntries.map(({ key, label }) => (
              <button key={key} className={selectedLayout === key ? "selected" : ""} onClick={() => setSelectedLayout(key)}>
                {label}
              </button>
            ))}
          </div>
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
          <WidgetLayoutFields
            layout={layoutForSelection(config, selectedLayout)}
            onChange={(key, value) => setLayoutSelection(config, setConfig, selectedLayout, key, value)}
          />
          {selectedLayout.startsWith("extra:") && config.extra_widgets[selectedLayout.slice("extra:".length)] && (
            <WidgetStyleFields
              style={config.extra_widgets[selectedLayout.slice("extra:".length)].style}
              onChange={(key, value) => setExtraWidgetStyle(config, setConfig, selectedLayout.slice("extra:".length), key, value)}
            />
          )}
        </Section>}

        {showPanel(activePage, "Dashboard", "Performance") && <Section icon={<Gauge />} title="Performance">
          <Segmented value={config.performance.mode} options={performanceModes} onChange={(value) => setPerformance(config, setConfig, value)} />
          <NumberField label="Render FPS" value={config.window.refresh_hz} onChange={(value) => setWindow(config, setConfig, "refresh_hz", value)} />
          <NumberField label="Sample ms" value={config.window.sample_ms} onChange={(value) => setWindow(config, setConfig, "sample_ms", value)} />
          <NumberField label="History samples" value={config.window.history_samples} onChange={(value) => setWindow(config, setConfig, "history_samples", value)} />
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
                  checked={config.widgets[key]}
                  onChange={(event) => setWidget(config, setConfig, key, event.target.checked)}
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
              const enabled = legacy ? config.widgets[legacy] : config.extra_widgets[widget.id]?.enabled ?? false;
              return (
                <label className="widgetCatalogRow" key={widget.id}>
                  <input type="checkbox" checked={enabled} onChange={(event) => setCatalogWidget(widget.id, event.target.checked)} />
                  <span><strong>{widget.name}</strong><small>{widget.description} · {widget.data_requirement}</small></span>
                </label>
              );
            })}
          </div>
        </Section>}

        {showPanel(activePage, "Dashboard", "Appearance") && <Section icon={<Paintbrush />} title="Appearance">
          <Segmented value={config.style.theme} options={themes} onChange={(value) => setStyle(config, setConfig, "theme", value)} />
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
          <button className="primaryButton wide" onClick={() => setConfig(addCustomPreset(config))}>
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
                <button title="Apply preset" onClick={() => setConfig(applyCustomPreset(config, index))}>
                  Apply
                </button>
                <button title="Save current into preset" onClick={() => setConfig(saveCurrentIntoCustomPreset(config, index))}>
                  <Save size={16} />
                </button>
                <button title="Delete preset" onClick={() => setConfig(deleteCustomPreset(config, index))}>
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
            <button className="primaryButton" onClick={resetLayout} disabled={!defaultConfigState}>
              Reset layout
            </button>
            <button className="dangerButton" onClick={resetConfig}>
              Reset all
            </button>
          </div>
          <label className="field stack">
            <span>Config TOML</span>
            <textarea value={configText} onChange={(event) => setConfigText(event.target.value)} spellCheck={false} />
          </label>
        </Section>}
      </div>

      <footer className="status">{status}</footer>
    </main>
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
}) {
  const [drag, setDrag] = useState<{
    widget: LayoutSelection;
    startX: number;
    startY: number;
    layout: WidgetLayout;
    resize: boolean;
  } | null>(null);
  const scale = Math.min(1, 320 / props.config.window.width);
  const previewWidth = props.config.window.width * scale;
  const previewHeight = props.config.window.height * scale;

  function moveWidget(clientX: number, clientY: number) {
    if (!drag) {
      return;
    }
    const deltaX = (clientX - drag.startX) / scale;
    const deltaY = (clientY - drag.startY) / scale;
    const next = drag.resize
      ? {
          ...drag.layout,
          width: clamp(Math.round(drag.layout.width + deltaX), 48, props.config.window.width),
          height: clamp(Math.round(drag.layout.height + deltaY), 20, props.config.window.height),
        }
      : {
          ...drag.layout,
          x: Math.round(drag.layout.x + deltaX),
          y: Math.round(drag.layout.y + deltaY),
        };
    props.onLayoutChange(drag.widget, snapLayout(props.config, drag.widget, next));
  }

  return (
    <div className="previewWrap">
      <div
        className="preview"
        onPointerMove={(event) => moveWidget(event.clientX, event.clientY)}
        onPointerUp={() => setDrag(null)}
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
              className={`previewWidget ${props.selected === key ? "selected" : ""}`}
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
              title={label}
            >
              <span>{label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function layoutEntries(config: OverlayConfig): Array<{ key: LayoutSelection; label: string; layout: WidgetLayout }> {
  const legacy = layoutLabels.map(([key, label]) => ({ key, label, layout: config.layout[key] }));
  const extra = Object.entries(config.extra_widgets)
    .filter(([, widget]) => widget.enabled)
    .map(([id, widget]) => ({ key: `extra:${id}` as LayoutSelection, label: id.replace(/_/g, " "), layout: widget.layout }));
  return [...legacy, ...extra];
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
  return next;
}

function snapNumber(value: number, gridSize: number) {
  const size = [5, 10, 20].includes(gridSize) ? gridSize : 10;
  return Math.round(value / size) * size;
}

function snapToEdges(config: OverlayConfig, layout: WidgetLayout): WidgetLayout {
  const next = { ...layout };
  const distance = config.layout.snap_distance;
  if (Math.abs(next.x) <= distance) {
    next.x = 0;
  }
  if (Math.abs(next.y) <= distance) {
    next.y = 0;
  }
  if (Math.abs(config.window.width - (next.x + next.width)) <= distance) {
    next.x = config.window.width - next.width;
  }
  if (Math.abs(config.window.height - (next.y + next.height)) <= distance) {
    next.y = config.window.height - next.height;
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
    const nextRight = next.x + next.width;
    const nextBottom = next.y + next.height;
    const otherRight = other.x + other.width;
    const otherBottom = other.y + other.height;
    if (Math.abs(next.x - other.x) <= distance) {
      next.x = other.x;
    } else if (Math.abs(next.x - otherRight) <= distance) {
      next.x = otherRight;
    } else if (Math.abs(nextRight - other.x) <= distance) {
      next.x = other.x - next.width;
    } else if (Math.abs(nextRight - otherRight) <= distance) {
      next.x = otherRight - next.width;
    }
    if (Math.abs(next.y - other.y) <= distance) {
      next.y = other.y;
    } else if (Math.abs(next.y - otherBottom) <= distance) {
      next.y = otherBottom;
    } else if (Math.abs(nextBottom - other.y) <= distance) {
      next.y = other.y - next.height;
    } else if (Math.abs(nextBottom - otherBottom) <= distance) {
      next.y = otherBottom - next.height;
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
  onChange: <K extends keyof WidgetLayout>(key: K, value: WidgetLayout[K]) => void;
}) {
  return (
    <>
      <NumberField label="Widget X" value={props.layout.x} onChange={(value) => props.onChange("x", value)} />
      <NumberField label="Widget Y" value={props.layout.y} onChange={(value) => props.onChange("y", value)} />
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

function setWindow<K extends keyof WindowConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: WindowConfig[K]) {
  setConfig({ ...config, window: { ...config.window, [key]: value } });
}

function setStyle<K extends keyof StyleConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: StyleConfig[K]) {
  setConfig({ ...config, style: { ...config.style, [key]: value } });
}

function setTiming<K extends keyof TimingConfig>(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: K, value: TimingConfig[K]) {
  setConfig({ ...config, timing: { ...config.timing, [key]: value } });
}

function setWidget(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: keyof WidgetConfig, value: boolean) {
  setConfig({ ...config, widgets: { ...config.widgets, [key]: value } });
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

function setLayoutSelection<K extends keyof WidgetLayout>(
  config: OverlayConfig,
  setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>,
  selection: LayoutSelection,
  key: K,
  value: WidgetLayout[K],
) {
  const layout = layoutForSelection(config, selection);
  setConfig(updateLayoutSelection(config, selection, { ...layout, [key]: value }));
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

function setHotkey(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: keyof HotkeyConfig, value: string) {
  setConfig({ ...config, hotkeys: { ...config.hotkeys, [key]: value } });
}

function setPerformance(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, mode: string) {
  setConfig({ ...config, performance: { mode } });
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

function applyCustomPreset(config: OverlayConfig, index: number): OverlayConfig {
  const preset = config.presets.custom[index];
  if (!preset) {
    return config;
  }
  return applyProfile(config, preset.profile);
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

function applyProfile(config: OverlayConfig, profile: PresetProfileConfig): OverlayConfig {
  return {
    ...config,
    performance: { mode: profile.performance_mode },
    timing: {
      ...config.timing,
      reference_mode: profile.reference_mode,
      mini_sectors: profile.mini_sectors,
    },
    widgets: pickWidgets(profile),
    style: structuredClone(profile.style),
    units: structuredClone(profile.units),
    coaching: structuredClone(profile.coaching_config),
    layout: structuredClone(profile.layout),
    extra_widgets: structuredClone(profile.extra_widgets),
  };
}

function titleCase(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

export default App;
