export type WindowConfig = {
  x: number;
  y: number;
  width: number;
  height: number;
  refresh_hz: number;
  sample_ms: number;
  history_samples: number;
};

export type StyleConfig = {
  opacity: number;
  scale: number;
  line_thickness: number;
  border_radius: number;
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

export type WidgetConfig = {
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

export type WidgetDefinition = {
  id: string;
  name: string;
  category: string;
  description: string;
  data_requirement: string;
  status: "Real" | "Computed from official data" | "Unavailable in current LMU interface";
};

export type WidgetLayout = {
  x: number;
  y: number;
  width: number;
  height: number;
  locked: boolean;
  scale: number;
  opacity: number;
  z_index: number;
};

export type WidgetStyleConfig = {
  inherit_theme: boolean;
  show_background: boolean;
  background_color: string;
  show_border: boolean;
  border_color: string;
  border_width: number;
  border_radius: number;
  padding: number;
  font_scale: number;
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  show_title: boolean;
  title_text: string;
};

export type WidgetOptions = {
  cars_ahead: number;
  cars_behind: number;
  rows: number;
  same_class_only: boolean;
  show_position: boolean;
  show_laps: boolean;
  show_driver: boolean;
  show_car: boolean;
  show_class: boolean;
  show_gap: boolean;
  show_pit: boolean;
  show_average: boolean;
  show_last_lap: boolean;
  show_best_lap: boolean;
  show_estimated_laps: boolean;
  show_wear: boolean;
  show_brake_temperature: boolean;
  show_brake_pressure: boolean;
  shift_start_percent: number;
  shift_warning_percent: number;
  limiter_percent: number;
  shift_segments: number;
  tyre_temperature_mode: string;
};

export type WidgetInstanceConfig = {
  enabled: boolean;
  layout: WidgetLayout;
  style: WidgetStyleConfig;
  options: WidgetOptions;
};

export type LayoutConfig = {
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

export type LayoutWidgetKey =
  | "telemetry"
  | "inputs"
  | "lap_timing"
  | "timing"
  | "sectors"
  | "mini_sectors"
  | "coaching"
  | "performance";
export type LayoutSelection = LayoutWidgetKey | `extra:${string}`;

export type UnitsConfig = {
  speed: string;
  temperature: string;
  pressure: string;
  fuel: string;
};

export type CoachingConfig = {
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

export type TimingConfig = {
  reference_mode: string;
  mini_sectors: number;
  brake_threshold: number;
  throttle_threshold: number;
};

export type HotkeyConfig = {
  toggle_overlay: string;
  edit_mode: string;
  toggle_coaching: string;
  cycle_preset: string;
};

export type PerformanceConfig = { mode: string };

export type PresetProfileConfig = {
  performance_mode: string;
  reference_mode: string;
  mini_sectors: number;
  style: StyleConfig;
  units: UnitsConfig;
  coaching_config: CoachingConfig;
  layout: LayoutConfig;
  extra_widgets: Record<string, WidgetInstanceConfig>;
} & WidgetConfig;

export type PresetConfig = {
  practice: PresetProfileConfig;
  qualifying: PresetProfileConfig;
  race: PresetProfileConfig;
  endurance: PresetProfileConfig;
  minimal: PresetProfileConfig;
  custom: CustomPresetConfig[];
};

export type CustomPresetConfig = {
  name: string;
  profile: PresetProfileConfig;
};

export type OverlayConfig = {
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
  overlays: OverlayLayerConfig[];
};

export type OverlayLayerConfig = {
  id: string;
  name: string;
  enabled: boolean;
  window: WindowConfig;
  widgets: string[];
  layout_overrides: Record<string, WidgetLayout>;
};

export type LoadResponse = { path: string; config: OverlayConfig };
