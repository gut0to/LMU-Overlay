import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Gauge,
  LayoutGrid,
  Paintbrush,
  RotateCcw,
  Save,
  SlidersHorizontal,
  Timer,
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
  reference: string;
};

type WidgetConfig = {
  title: boolean;
  speed_gear_rpm: boolean;
  pedals: boolean;
  steering: boolean;
  lap_info: boolean;
  input_history: boolean;
  delta_timing: boolean;
  ghost_inputs: boolean;
  coaching: boolean;
  performance_monitor: boolean;
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
};

type PerformanceConfig = {
  mode: string;
};

type PresetProfileConfig = {
  performance_mode: string;
  reference_mode: string;
  mini_sectors: number;
} & WidgetConfig;

type PresetConfig = {
  practice: PresetProfileConfig;
  qualifying: PresetProfileConfig;
  race: PresetProfileConfig;
};

type OverlayConfig = {
  window: WindowConfig;
  style: StyleConfig;
  widgets: WidgetConfig;
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

const presetNames = ["practice", "qualifying", "race"] as const;
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
  ["last_lap", "Last Lap"],
];

const widgetLabels: Array<[keyof WidgetConfig, string]> = [
  ["title", "Title"],
  ["speed_gear_rpm", "Speed, gear, RPM"],
  ["pedals", "Pedals"],
  ["steering", "Steering"],
  ["lap_info", "Lap info"],
  ["input_history", "Input history"],
  ["delta_timing", "Delta timing"],
  ["ghost_inputs", "Ghost inputs"],
  ["coaching", "Coaching"],
  ["performance_monitor", "Performance monitor"],
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
  ["delta_gain", "Delta gain"],
  ["delta_loss", "Delta loss"],
  ["reference", "Reference"],
];

function App() {
  const [config, setConfig] = useState<OverlayConfig | null>(null);
  const [path, setPath] = useState("");
  const [status, setStatus] = useState("Loading config");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    void loadConfig();
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

  function applyPreset(name: keyof PresetConfig) {
    setConfig((current) => {
      if (!current) {
        return current;
      }
      const preset = current.presets[name];
      return {
        ...current,
        performance: { mode: preset.performance_mode },
        timing: {
          ...current.timing,
          reference_mode: preset.reference_mode,
          mini_sectors: preset.mini_sectors,
        },
        widgets: pickWidgets(preset),
      };
    });
  }

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
          <button className="iconButton" title="Reload config" onClick={loadConfig}>
            <RotateCcw size={18} />
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

      <div className="grid">
        <Section icon={<LayoutGrid />} title="Layout">
          <NumberField label="X" value={config.window.x} onChange={(value) => setWindow(config, setConfig, "x", value)} />
          <NumberField label="Y" value={config.window.y} onChange={(value) => setWindow(config, setConfig, "y", value)} />
          <NumberField label="Width" value={config.window.width} onChange={(value) => setWindow(config, setConfig, "width", value)} />
          <NumberField label="Height" value={config.window.height} onChange={(value) => setWindow(config, setConfig, "height", value)} />
          <RangeField label="Scale" min={0.65} max={1.75} step={0.05} value={config.style.scale} onChange={(value) => setStyle(config, setConfig, "scale", value)} />
          <RangeField label="Opacity" min={32} max={255} step={1} value={config.style.opacity} onChange={(value) => setStyle(config, setConfig, "opacity", value)} />
        </Section>

        <Section icon={<Gauge />} title="Performance">
          <Segmented value={config.performance.mode} options={performanceModes} onChange={(value) => setPerformance(config, setConfig, value)} />
          <NumberField label="Render FPS" value={config.window.refresh_hz} onChange={(value) => setWindow(config, setConfig, "refresh_hz", value)} />
          <NumberField label="Sample ms" value={config.window.sample_ms} onChange={(value) => setWindow(config, setConfig, "sample_ms", value)} />
          <NumberField label="History samples" value={config.window.history_samples} onChange={(value) => setWindow(config, setConfig, "history_samples", value)} />
          <RangeField label="Line thickness" min={1} max={8} step={1} value={config.style.line_thickness} onChange={(value) => setStyle(config, setConfig, "line_thickness", value)} />
        </Section>

        <Section icon={<Timer />} title="Timing">
          <Segmented value={config.timing.reference_mode} options={referenceModes} onChange={(value) => setTiming(config, setConfig, "reference_mode", value)} />
          <NumberField label="Mini sectors" value={config.timing.mini_sectors} onChange={(value) => setTiming(config, setConfig, "mini_sectors", value)} />
          <RangeField label="Brake threshold" min={0.01} max={1} step={0.01} value={config.timing.brake_threshold} onChange={(value) => setTiming(config, setConfig, "brake_threshold", value)} />
          <RangeField label="Throttle threshold" min={0.01} max={1} step={0.01} value={config.timing.throttle_threshold} onChange={(value) => setTiming(config, setConfig, "throttle_threshold", value)} />
        </Section>

        <Section icon={<SlidersHorizontal />} title="Widgets">
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
        </Section>

        <Section icon={<Paintbrush />} title="Colors">
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
        </Section>

        <Section icon={<Activity />} title="Hotkeys">
          <TextField label="Show or hide overlay" value={config.hotkeys.toggle_overlay} onChange={(value) => setHotkey(config, setConfig, "toggle_overlay", value)} />
          <TextField label="Edit mode" value={config.hotkeys.edit_mode} onChange={(value) => setHotkey(config, setConfig, "edit_mode", value)} />
        </Section>
      </div>

      <footer className="status">{status}</footer>
    </main>
  );
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

function TextField(props: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <label className="field">
      <span>{props.label}</span>
      <input value={props.value} onChange={(event) => props.onChange(event.target.value.toUpperCase())} />
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

function setHotkey(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, key: keyof HotkeyConfig, value: string) {
  setConfig({ ...config, hotkeys: { ...config.hotkeys, [key]: value } });
}

function setPerformance(config: OverlayConfig, setConfig: React.Dispatch<React.SetStateAction<OverlayConfig | null>>, mode: string) {
  setConfig({ ...config, performance: { mode } });
}

function pickWidgets(profile: PresetProfileConfig): WidgetConfig {
  return Object.fromEntries(widgetLabels.map(([key]) => [key, profile[key]])) as WidgetConfig;
}

function titleCase(value: string) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

export default App;
