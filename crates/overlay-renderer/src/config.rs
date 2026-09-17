use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

use serde::{Deserialize, Serialize};

const CURRENT_CONFIG_VERSION: u32 = 7;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayConfig {
    pub config_version: u32,
    pub window: WindowConfig,
    pub style: StyleConfig,
    pub widgets: WidgetConfig,
    pub extra_widgets: BTreeMap<String, WidgetInstanceConfig>,
    pub layout: LayoutConfig,
    pub units: UnitsConfig,
    pub coaching: CoachingConfig,
    pub timing: TimingConfig,
    pub hotkeys: HotkeyConfig,
    pub performance: PerformanceConfig,
    pub presets: PresetConfig,
    pub overlays: Vec<OverlayLayerConfig>,
}

impl OverlayConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&text)?;
        let original_version = config.config_version;
        if original_version > CURRENT_CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: original_version,
                supported: CURRENT_CONFIG_VERSION,
            });
        }
        let needs_migration = original_version < CURRENT_CONFIG_VERSION;
        let missing_overlay_layers = config.overlays.is_empty();
        if needs_migration {
            write_migration_backup(path, original_version, &text)?;
            migrate_config(&mut config, original_version);
        }
        config.normalize();
        if needs_migration || missing_overlay_layers {
            atomic_write(path, toml::to_string_pretty(&config)?)?;
        }
        Ok(config)
    }

    pub fn save_default(path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            fs::write(path, default_config_text())?;
        }
        Ok(())
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        self.validate_for_save()?;
        self.normalize();
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn revision(path: impl AsRef<Path>) -> Result<u64, ConfigError> {
        let bytes = fs::read(path)?;
        Ok(config_revision(&bytes))
    }

    pub fn save_if_revision(
        &mut self,
        path: impl AsRef<Path>,
        expected_revision: u64,
    ) -> Result<u64, ConfigError> {
        self.validate_for_save()?;
        let path = path.as_ref();
        let current_revision = Self::revision(path)?;
        if current_revision != expected_revision {
            return Err(ConfigError::Conflict {
                expected: expected_revision,
                actual: current_revision,
            });
        }
        self.normalize();
        let text = toml::to_string_pretty(self)?;
        let next_revision = config_revision(text.as_bytes());
        atomic_write(path, text)?;
        Ok(next_revision)
    }

    /// Validate a candidate before any persistence operation can replace the
    /// user's working configuration. Normalization is applied to a clone so
    /// recoverable legacy/range issues remain repairable while future schemas
    /// are rejected without touching the destination file.
    pub fn validate_for_save(&self) -> Result<(), ConfigError> {
        if self.config_version > CURRENT_CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: self.config_version,
                supported: CURRENT_CONFIG_VERSION,
            });
        }
        let mut normalized = self.clone();
        normalized.normalize();
        if normalized.config_version > CURRENT_CONFIG_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                found: normalized.config_version,
                supported: CURRENT_CONFIG_VERSION,
            });
        }
        normalized.hotkeys.validate()?;
        toml::to_string_pretty(&normalized)?;
        Ok(())
    }

    pub fn normalize(&mut self) {
        if self.config_version == 0 {
            self.config_version = CURRENT_CONFIG_VERSION;
        }
        self.window.width = self.window.width.clamp(280, 1200);
        self.window.height = self.window.height.clamp(140, 800);
        match self.performance.mode.as_str() {
            "eco" => {
                self.window.refresh_hz = 30;
                self.window.sample_ms = 20;
            }
            "high_refresh" => {
                self.window.refresh_hz = 120;
                self.window.sample_ms = 10;
            }
            "normal" => {
                self.window.refresh_hz = 60;
                self.window.sample_ms = 10;
            }
            _ => {}
        }
        self.window.refresh_hz = self.window.refresh_hz.clamp(15, 144);
        self.window.sample_ms = self.window.sample_ms.clamp(5, 250);
        self.window.history_samples = self.window.history_samples.clamp(16, 900);
        self.style.opacity = self.style.opacity.clamp(32, 255);
        self.style.scale = self.style.scale.clamp(0.65, 1.75);
        self.style.line_thickness = self.style.line_thickness.clamp(1, 8);
        self.style.border_radius = self.style.border_radius.clamp(0, 32);
        self.style.font_size = self.style.font_size.clamp(8, 36);
        self.style.font_weight = self.style.font_weight.clamp(100, 900);
        self.style.large_number_size = self.style.large_number_size.clamp(12, 72);
        self.layout.normalize();
        self.extra_widgets.retain(|id, _| {
            crate::widgets::WIDGET_CATALOG
                .iter()
                .any(|definition| definition.id == id)
        });
        for (index, definition) in crate::widgets::WIDGET_CATALOG.iter().enumerate() {
            self.extra_widgets
                .entry(definition.id.to_string())
                .or_insert_with(|| WidgetInstanceConfig::disabled(index));
        }
        for widget in self.extra_widgets.values_mut() {
            widget.layout.normalize();
            widget.style.normalize();
        }
        self.units.normalize();
        self.coaching.normalize();
        self.timing.mini_sectors = self.timing.mini_sectors.clamp(1, 200);
        self.timing.brake_threshold = self.timing.brake_threshold.clamp(0.01, 1.0);
        self.timing.throttle_threshold = self.timing.throttle_threshold.clamp(0.01, 1.0);
        self.presets.normalize();
        normalize_overlay_layers(self);
    }

    /// Applies the next built-in runtime preset without changing the hotkey
    /// bindings or the configured overlay surfaces.
    pub fn cycle_preset(&mut self) {
        let next = match (
            self.performance.mode.as_str(),
            self.timing.reference_mode.as_str(),
        ) {
            ("normal", "last_lap") => self.presets.qualifying.clone(),
            ("high_refresh", "personal_best") => self.presets.race.clone(),
            ("eco", "session_best") => self.presets.endurance.clone(),
            ("eco", _) => self.presets.minimal.clone(),
            _ => self.presets.practice.clone(),
        };
        self.performance.mode = next.performance_mode;
        self.timing.reference_mode = next.reference_mode;
        self.timing.mini_sectors = next.mini_sectors;
        self.style = next.style;
        self.units = next.units;
        self.coaching = next.coaching_config;
        self.layout = next.layout;
        self.extra_widgets = next.extra_widgets;
        self.widgets.title = next.title;
        self.widgets.speed_gear_rpm = next.speed_gear_rpm;
        self.widgets.pedals = next.pedals;
        self.widgets.steering = next.steering;
        self.widgets.lap_info = next.lap_info;
        self.widgets.lap_timing = next.lap_timing;
        self.widgets.sectors = next.sectors;
        self.widgets.mini_sector_widget = next.mini_sector_widget;
        self.widgets.input_history = next.input_history;
        self.widgets.delta_timing = next.delta_timing;
        self.widgets.ghost_inputs = next.ghost_inputs;
        self.widgets.coaching = next.coaching;
        self.widgets.performance_monitor = next.performance_monitor;
        self.normalize();
    }

    /// Returns the runtime view for one independent overlay surface.
    /// The persisted config remains the source of truth; each process only
    /// receives the widgets and window belonging to its selected layer.
    pub fn for_overlay_layer(&self, layer_id: Option<&str>) -> Result<Self, ConfigError> {
        let Some(layer_id) = layer_id else {
            return Ok(self.clone());
        };
        let Some(layer) = self.overlays.iter().find(|layer| layer.id == layer_id) else {
            return Err(ConfigError::UnknownOverlayLayer(layer_id.to_string()));
        };

        let mut next = self.clone();
        next.window = layer.window.clone();
        let has = |id: &str| layer.widgets.iter().any(|widget| widget == id);
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
        for (id, widget) in &mut next.extra_widgets {
            widget.enabled = widget.enabled && has(id);
        }

        for (id, layout) in &layer.layout_overrides {
            match id.as_str() {
                "telemetry" => next.layout.telemetry = layout.clone(),
                "inputs" => next.layout.inputs = layout.clone(),
                "lap_timing" => next.layout.lap_timing = layout.clone(),
                "timing" => next.layout.timing = layout.clone(),
                "sectors" => next.layout.sectors = layout.clone(),
                "mini_sectors" => next.layout.mini_sectors = layout.clone(),
                "coaching" => next.layout.coaching = layout.clone(),
                "performance" => next.layout.performance = layout.clone(),
                _ => {
                    if let Some(widget) = next.extra_widgets.get_mut(id) {
                        widget.layout = layout.clone();
                    }
                }
            }
        }
        next.overlays.clear();
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayLayerConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub window: WindowConfig,
    pub widgets: Vec<String>,
    pub layout_overrides: BTreeMap<String, WidgetLayout>,
}

impl Default for OverlayLayerConfig {
    fn default() -> Self {
        Self {
            id: "main".to_string(),
            name: "Main overlay".to_string(),
            enabled: true,
            window: WindowConfig::default(),
            widgets: default_overlay_widget_ids(),
            layout_overrides: BTreeMap::new(),
        }
    }
}

fn default_overlay_widget_ids() -> Vec<String> {
    crate::widgets::OVERLAY_SURFACE_IDS
        .iter()
        .copied()
        .map(str::to_string)
        .collect()
}

fn normalize_overlay_layers(config: &mut OverlayConfig) {
    if config.overlays.is_empty() {
        config.overlays.push(OverlayLayerConfig::default());
    }
    let mut used_ids = std::collections::BTreeSet::new();
    let allowed: std::collections::BTreeSet<&str> = crate::widgets::OVERLAY_SURFACE_IDS
        .iter()
        .copied()
        .collect();
    for (index, overlay) in config.overlays.iter_mut().enumerate() {
        overlay.id = overlay.id.trim().to_ascii_lowercase();
        if overlay.id.is_empty() || !used_ids.insert(overlay.id.clone()) {
            overlay.id = format!("overlay-{}", index + 1);
            used_ids.insert(overlay.id.clone());
        }
        if overlay.name.trim().is_empty() {
            overlay.name = format!("Overlay {}", index + 1);
        }
        overlay.window.width = overlay.window.width.clamp(280, 1200);
        overlay.window.height = overlay.window.height.clamp(140, 800);
        overlay.window.refresh_hz = overlay.window.refresh_hz.clamp(15, 144);
        overlay.window.sample_ms = overlay.window.sample_ms.clamp(5, 250);
        overlay.widgets.retain(|id| allowed.contains(id.as_str()));
        overlay.widgets.sort();
        overlay.widgets.dedup();
        overlay.layout_overrides.retain(|id, layout| {
            allowed.contains(id.as_str()) && {
                layout.normalize();
                true
            }
        });
    }
}

fn migrate_config(config: &mut OverlayConfig, from_version: u32) {
    match from_version {
        0 | 1 => {
            migrate_v1_to_v2(config);
            migrate_v2_to_v3(config);
            migrate_v3_to_v4(config);
            migrate_v4_to_v5(config);
            migrate_v5_to_v6(config);
            migrate_v6_to_v7(config);
        }
        2 => {
            migrate_v2_to_v3(config);
            migrate_v3_to_v4(config);
            migrate_v4_to_v5(config);
            migrate_v5_to_v6(config);
            migrate_v6_to_v7(config);
        }
        3 => {
            migrate_v3_to_v4(config);
            migrate_v4_to_v5(config);
            migrate_v5_to_v6(config);
            migrate_v6_to_v7(config);
        }
        4 => {
            migrate_v4_to_v5(config);
            migrate_v5_to_v6(config);
            migrate_v6_to_v7(config);
        }
        5 => {
            migrate_v5_to_v6(config);
            migrate_v6_to_v7(config);
        }
        6 => migrate_v6_to_v7(config),
        _ => config.config_version = CURRENT_CONFIG_VERSION,
    }
}

fn migrate_v1_to_v2(config: &mut OverlayConfig) {
    if config.hotkeys.toggle_overlay.trim().is_empty() {
        config.hotkeys.toggle_overlay = "F9".to_string();
    }
    if config.hotkeys.edit_mode.trim().is_empty() {
        config.hotkeys.edit_mode = "F10".to_string();
    }
    config.config_version = 2;
}

fn migrate_v2_to_v3(config: &mut OverlayConfig) {
    if config.hotkeys.toggle_coaching.trim().is_empty() {
        config.hotkeys.toggle_coaching = "Shift+F10".to_string();
    }
    if config.hotkeys.cycle_preset.trim().is_empty() {
        config.hotkeys.cycle_preset = "Ctrl+Shift+F9".to_string();
    }
    config.config_version = 3;
}

fn migrate_v3_to_v4(config: &mut OverlayConfig) {
    if config.units.temperature.trim().is_empty() {
        config.units.temperature = "celsius".to_string();
    }
    if config.units.pressure.trim().is_empty() {
        config.units.pressure = "kpa".to_string();
    }
    if config.units.fuel.trim().is_empty() {
        config.units.fuel = "liters".to_string();
    }
    config.config_version = 4;
}

fn migrate_v4_to_v5(config: &mut OverlayConfig) {
    config.presets.normalize();
    config.config_version = 5;
}

fn migrate_v5_to_v6(config: &mut OverlayConfig) {
    for widget in config.extra_widgets.values_mut() {
        widget.style.normalize();
    }
    for profile in [
        &mut config.presets.practice,
        &mut config.presets.qualifying,
        &mut config.presets.race,
        &mut config.presets.endurance,
        &mut config.presets.minimal,
    ] {
        profile.normalize();
    }
    for preset in &mut config.presets.custom {
        preset.profile.normalize();
    }
    config.config_version = 6;
}

fn migrate_v6_to_v7(config: &mut OverlayConfig) {
    // WidgetOptions uses serde defaults, so existing widget instances remain
    // compatible while gaining the new per-widget controls.
    config.config_version = 7;
}

fn write_migration_backup(path: &Path, version: u32, text: &str) -> io::Result<()> {
    let backup_path = path.with_extension(format!("toml.v{version}.bak"));
    fs::write(backup_path, text)
}

fn atomic_write(path: &Path, text: String) -> io::Result<()> {
    let temp_path = temp_config_path(path);
    fs::write(&temp_path, text)?;
    replace_file(&temp_path, path)
}

fn temp_config_path(path: &Path) -> PathBuf {
    let mut temp_path = path.to_path_buf();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!("{value}.tmp"))
        .unwrap_or_else(|| "tmp".to_string());
    temp_path.set_extension(format!(
        "{extension}.{}.{}",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    temp_path
}

fn config_revision(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(windows)]
fn replace_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    let temp = wide_path(temp_path);
    let target = wide_path(path);
    let replaced = unsafe {
        MoveFileExW(
            temp.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temp_path, path)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowConfig {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub refresh_hz: u64,
    pub sample_ms: u64,
    pub history_samples: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            x: 40,
            y: 40,
            width: 420,
            height: 230,
            refresh_hz: 60,
            sample_ms: 10,
            history_samples: 180,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StyleConfig {
    pub opacity: u8,
    pub scale: f64,
    pub line_thickness: i32,
    pub border_radius: i32,
    pub background: String,
    pub border: String,
    pub primary_text: String,
    pub secondary_text: String,
    pub throttle: String,
    pub brake: String,
    pub clutch: String,
    pub steering: String,
    pub delta_gain: String,
    pub delta_loss: String,
    pub delta_neutral: String,
    pub reference: String,
    pub rpm: String,
    pub coaching_warning: String,
    pub coaching_positive: String,
    pub font_family: String,
    pub font_size: i32,
    pub font_weight: i32,
    pub large_number_size: i32,
    pub theme: String,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            opacity: 230,
            scale: 1.0,
            line_thickness: 2,
            border_radius: 8,
            background: "#202020".to_string(),
            border: "#666666".to_string(),
            primary_text: "#ffffff".to_string(),
            secondary_text: "#d0d0d0".to_string(),
            throttle: "#44dd22".to_string(),
            brake: "#ee4422".to_string(),
            clutch: "#22dddd".to_string(),
            steering: "#eeeeee".to_string(),
            delta_gain: "#44dd22".to_string(),
            delta_loss: "#ee4422".to_string(),
            delta_neutral: "#f2bc57".to_string(),
            reference: "#aaaaaa".to_string(),
            rpm: "#f2bc57".to_string(),
            coaching_warning: "#f2bc57".to_string(),
            coaching_positive: "#44dd22".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 14,
            font_weight: 500,
            large_number_size: 24,
            theme: "hashoverlay_default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UnitsConfig {
    pub speed: String,
    pub temperature: String,
    pub pressure: String,
    pub fuel: String,
}

impl UnitsConfig {
    fn normalize(&mut self) {
        if !matches!(self.speed.as_str(), "kmh" | "mph") {
            self.speed = "kmh".to_string();
        }
        if !matches!(self.temperature.as_str(), "celsius" | "fahrenheit") {
            self.temperature = "celsius".to_string();
        }
        if !matches!(self.pressure.as_str(), "kpa" | "psi") {
            self.pressure = "kpa".to_string();
        }
        if !matches!(self.fuel.as_str(), "liters" | "gallons") {
            self.fuel = "liters".to_string();
        }
    }
}

impl Default for UnitsConfig {
    fn default() -> Self {
        Self {
            speed: "kmh".to_string(),
            temperature: "celsius".to_string(),
            pressure: "kpa".to_string(),
            fuel: "liters".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CoachingConfig {
    pub mode: String,
    pub brake_timing: bool,
    pub throttle_timing: bool,
    pub input_match: bool,
    pub speed: bool,
    pub gear: bool,
    pub speed_threshold_kph: f64,
    pub timing_deadband_m: f64,
    pub event_match_tolerance_m: f64,
    pub max_hints: u8,
}

impl CoachingConfig {
    fn normalize(&mut self) {
        if !matches!(self.mode.as_str(), "off" | "race" | "practice" | "attack") {
            self.mode = "practice".to_string();
        }
        self.speed_threshold_kph = self.speed_threshold_kph.clamp(1.0, 40.0);
        self.timing_deadband_m = self.timing_deadband_m.clamp(0.0, 50.0);
        self.event_match_tolerance_m = self.event_match_tolerance_m.clamp(10.0, 500.0);
        self.max_hints = self.max_hints.clamp(1, 6);
        if self.mode == "off" {
            self.brake_timing = false;
            self.throttle_timing = false;
            self.input_match = false;
            self.speed = false;
            self.gear = false;
        }
    }
}

impl Default for CoachingConfig {
    fn default() -> Self {
        Self {
            mode: "practice".to_string(),
            brake_timing: true,
            throttle_timing: true,
            input_match: true,
            speed: true,
            gear: true,
            speed_threshold_kph: 5.0,
            timing_deadband_m: 3.0,
            event_match_tolerance_m: 80.0,
            max_hints: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidgetConfig {
    pub title: bool,
    pub speed_gear_rpm: bool,
    pub pedals: bool,
    pub steering: bool,
    pub lap_info: bool,
    pub lap_timing: bool,
    pub sectors: bool,
    pub mini_sector_widget: bool,
    pub input_history: bool,
    pub delta_timing: bool,
    pub ghost_inputs: bool,
    pub coaching: bool,
    pub performance_monitor: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            title: true,
            speed_gear_rpm: true,
            pedals: true,
            steering: true,
            lap_info: true,
            lap_timing: true,
            sectors: true,
            mini_sector_widget: true,
            input_history: true,
            delta_timing: true,
            ghost_inputs: true,
            coaching: true,
            performance_monitor: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    pub lock_all: bool,
    pub snap_to_edges: bool,
    pub snap_to_grid: bool,
    pub snap_to_widgets: bool,
    pub grid_size: i32,
    pub snap_distance: i32,
    pub telemetry: WidgetLayout,
    pub inputs: WidgetLayout,
    pub lap_timing: WidgetLayout,
    pub timing: WidgetLayout,
    pub sectors: WidgetLayout,
    pub mini_sectors: WidgetLayout,
    pub coaching: WidgetLayout,
    pub performance: WidgetLayout,
}

impl LayoutConfig {
    fn normalize(&mut self) {
        if !matches!(self.grid_size, 5 | 10 | 20) {
            self.grid_size = 10;
        }
        self.snap_distance = self.snap_distance.clamp(0, 64);
        self.telemetry.normalize();
        self.inputs.normalize();
        self.lap_timing.normalize();
        self.timing.normalize();
        self.sectors.normalize();
        self.mini_sectors.normalize();
        self.coaching.normalize();
        self.performance.normalize();
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            lock_all: false,
            snap_to_edges: true,
            snap_to_grid: false,
            snap_to_widgets: true,
            grid_size: 10,
            snap_distance: 12,
            telemetry: WidgetLayout {
                x: 14,
                y: 10,
                width: 392,
                height: 52,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 10,
            },
            inputs: WidgetLayout {
                x: 14,
                y: 68,
                width: 240,
                height: 88,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 20,
            },
            lap_timing: WidgetLayout {
                x: 266,
                y: 68,
                width: 140,
                height: 42,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 30,
            },
            timing: WidgetLayout {
                x: 266,
                y: 114,
                width: 140,
                height: 42,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 40,
            },
            sectors: WidgetLayout {
                x: 14,
                y: 164,
                width: 190,
                height: 42,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 50,
            },
            mini_sectors: WidgetLayout {
                x: 210,
                y: 164,
                width: 196,
                height: 42,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 60,
            },
            coaching: WidgetLayout {
                x: 14,
                y: 212,
                width: 392,
                height: 58,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 70,
            },
            performance: WidgetLayout {
                x: 14,
                y: 278,
                width: 392,
                height: 20,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 80,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub locked: bool,
    pub scale: f64,
    pub opacity: f64,
    pub z_index: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidgetInstanceConfig {
    pub enabled: bool,
    pub layout: WidgetLayout,
    pub style: WidgetStyleConfig,
    pub options: WidgetOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Widget options evolve independently from the top-level config. Ignore newer
// option fields so a Settings/overlay version mismatch cannot reject the whole
// configuration; known fields still use their defaults and are normalized.
#[serde(default)]
pub struct WidgetOptions {
    pub cars_ahead: u8,
    pub cars_behind: u8,
    pub rows: u8,
    pub same_class_only: bool,
    pub show_position: bool,
    pub show_laps: bool,
    pub show_driver: bool,
    pub show_car: bool,
    pub show_class: bool,
    pub show_gap: bool,
    pub show_pit: bool,
    pub show_average: bool,
    pub show_last_lap: bool,
    pub show_best_lap: bool,
    pub show_estimated_laps: bool,
    pub show_wear: bool,
    pub show_brake_temperature: bool,
    pub show_brake_pressure: bool,
    pub show_bias: bool,
    pub shift_start_percent: u8,
    pub shift_warning_percent: u8,
    pub limiter_percent: u8,
    pub shift_segments: u8,
    pub tyre_temperature_mode: String,
}

impl Default for WidgetOptions {
    fn default() -> Self {
        Self {
            cars_ahead: 2,
            cars_behind: 2,
            rows: 8,
            same_class_only: false,
            show_position: true,
            show_laps: true,
            show_driver: true,
            show_car: false,
            show_class: false,
            show_gap: true,
            show_pit: true,
            show_average: true,
            show_last_lap: true,
            show_best_lap: true,
            show_estimated_laps: true,
            show_wear: true,
            show_brake_temperature: true,
            show_brake_pressure: true,
            show_bias: true,
            shift_start_percent: 70,
            shift_warning_percent: 85,
            limiter_percent: 95,
            shift_segments: 10,
            tyre_temperature_mode: "surface_average".to_string(),
        }
    }
}

impl WidgetOptions {
    fn normalize(&mut self) {
        self.cars_ahead = self.cars_ahead.min(8);
        self.cars_behind = self.cars_behind.min(8);
        self.rows = self.rows.clamp(1, 20);
        self.shift_start_percent = self.shift_start_percent.min(100);
        self.shift_warning_percent = self
            .shift_warning_percent
            .clamp(self.shift_start_percent, 100);
        self.limiter_percent = self.limiter_percent.clamp(self.shift_warning_percent, 100);
        self.shift_segments = self.shift_segments.clamp(4, 20);
        if !matches!(
            self.tyre_temperature_mode.as_str(),
            "surface_average" | "surface_lcr" | "carcass" | "inner_layer"
        ) {
            self.tyre_temperature_mode = "surface_average".to_string();
        }
    }
}

impl WidgetInstanceConfig {
    fn disabled(index: usize) -> Self {
        let column = (index % 3) as i32;
        let row = (index / 3) as i32;
        Self {
            enabled: false,
            layout: WidgetLayout {
                x: 16 + column * 142,
                y: 16 + row * 62,
                width: 132,
                height: 52,
                locked: false,
                scale: 1.0,
                opacity: 1.0,
                z_index: 100 + index as i32,
            },
            style: WidgetStyleConfig::default(),
            options: WidgetOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidgetStyleConfig {
    pub inherit_theme: bool,
    pub show_background: bool,
    pub background_color: String,
    pub show_border: bool,
    pub border_color: String,
    pub border_width: i32,
    pub border_radius: i32,
    pub padding: i32,
    pub font_scale: f64,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: String,
    pub show_title: bool,
    pub title_text: String,
}

impl WidgetStyleConfig {
    fn normalize(&mut self) {
        self.border_width = self.border_width.clamp(0, 8);
        self.border_radius = self.border_radius.clamp(0, 32);
        self.padding = self.padding.clamp(0, 48);
        self.font_scale = self.font_scale.clamp(0.5, 2.0);
        self.title_text.truncate(80);
    }
}

impl Default for WidgetStyleConfig {
    fn default() -> Self {
        Self {
            inherit_theme: true,
            show_background: true,
            background_color: "#202020".to_string(),
            show_border: true,
            border_color: "#666666".to_string(),
            border_width: 1,
            border_radius: 8,
            padding: 8,
            font_scale: 1.0,
            primary_color: "#ffffff".to_string(),
            secondary_color: "#d0d0d0".to_string(),
            accent_color: "#f2bc57".to_string(),
            show_title: false,
            title_text: String::new(),
        }
    }
}

impl Default for WidgetInstanceConfig {
    fn default() -> Self {
        Self::disabled(0)
    }
}

impl WidgetLayout {
    fn normalize(&mut self) {
        self.x = self.x.clamp(-2000, 8000);
        self.y = self.y.clamp(-2000, 8000);
        self.width = self.width.clamp(48, 1600);
        self.height = self.height.clamp(20, 1000);
        self.scale = self.scale.clamp(0.5, 2.0);
        self.opacity = self.opacity.clamp(0.1, 1.0);
        self.z_index = self.z_index.clamp(-1000, 1000);
    }
}

impl Default for WidgetLayout {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 160,
            height: 80,
            locked: false,
            scale: 1.0,
            opacity: 1.0,
            z_index: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimingConfig {
    pub reference_mode: String,
    pub mini_sectors: u16,
    pub brake_threshold: f64,
    pub throttle_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeyConfig {
    pub toggle_overlay: String,
    pub edit_mode: String,
    pub toggle_coaching: String,
    pub cycle_preset: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle_overlay: "F9".to_string(),
            edit_mode: "F10".to_string(),
            toggle_coaching: "Shift+F10".to_string(),
            cycle_preset: "Ctrl+Shift+F9".to_string(),
        }
    }
}

impl HotkeyConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        let bindings = [
            ("toggle_overlay", self.toggle_overlay.as_str()),
            ("edit_mode", self.edit_mode.as_str()),
            ("toggle_coaching", self.toggle_coaching.as_str()),
            ("cycle_preset", self.cycle_preset.as_str()),
        ];
        let mut seen = BTreeMap::new();

        for (field, binding) in bindings {
            let normalized =
                normalize_hotkey(binding).ok_or_else(|| ConfigError::InvalidHotkey {
                    field,
                    binding: binding.to_string(),
                })?;
            if let Some(first_field) = seen.insert(normalized, field) {
                return Err(ConfigError::DuplicateHotkey {
                    first_field,
                    second_field: field,
                    binding: binding.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn normalize_hotkey(value: &str) -> Option<String> {
    let mut modifiers = Vec::new();
    let mut function_key = None;
    for part in value
        .split('+')
        .map(|part| part.trim().to_ascii_uppercase())
    {
        match part.as_str() {
            "CTRL" | "CONTROL" if !modifiers.contains(&"CTRL") => modifiers.push("CTRL"),
            "SHIFT" if !modifiers.contains(&"SHIFT") => modifiers.push("SHIFT"),
            "ALT" if !modifiers.contains(&"ALT") => modifiers.push("ALT"),
            key if function_key.is_none() => {
                let number = key
                    .strip_prefix('F')
                    .and_then(|number| number.parse::<u8>().ok())?;
                if !(1..=12).contains(&number) {
                    return None;
                }
                function_key = Some(format!("F{number}"));
            }
            _ => return None,
        }
    }
    function_key.map(|key| {
        modifiers.sort_unstable();
        modifiers
            .into_iter()
            .chain(std::iter::once(key.as_str()))
            .collect::<Vec<_>>()
            .join("+")
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PerformanceConfig {
    pub mode: String,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            mode: "normal".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PresetConfig {
    pub practice: PresetProfileConfig,
    pub qualifying: PresetProfileConfig,
    pub race: PresetProfileConfig,
    pub endurance: PresetProfileConfig,
    pub minimal: PresetProfileConfig,
    pub custom: Vec<CustomPresetConfig>,
}

impl PresetConfig {
    fn normalize(&mut self) {
        self.practice.normalize();
        self.qualifying.normalize();
        self.race.normalize();
        self.endurance.normalize();
        self.minimal.normalize();
        self.custom.truncate(32);
        for preset in &mut self.custom {
            preset.profile.normalize();
        }
    }
}

impl Default for PresetConfig {
    fn default() -> Self {
        let mut race = PresetProfileConfig::race().with_extra_widgets(&[
            "position",
            "relative",
            "standings",
            "fuel",
            "flags",
        ]);
        arrange_preset_profile_layout(&mut race);
        let mut practice = PresetProfileConfig::practice()
            .with_extra_widgets(&["tyres", "brakes", "fuel", "engine", "weather"]);
        arrange_preset_profile_layout(&mut practice);
        let mut qualifying =
            PresetProfileConfig::qualifying().with_extra_widgets(&["tyres", "brakes", "engine"]);
        arrange_preset_profile_layout(&mut qualifying);
        let mut endurance = PresetProfileConfig::endurance().with_extra_widgets(&[
            "position",
            "relative",
            "standings",
            "fuel",
            "energy",
            "tyres",
            "brakes",
            "engine",
            "weather",
            "damage",
        ]);
        arrange_preset_profile_layout(&mut endurance);
        let mut minimal = PresetProfileConfig::minimal();
        arrange_preset_profile_layout(&mut minimal);
        Self {
            practice,
            qualifying,
            race,
            endurance,
            minimal,
            custom: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CustomPresetConfig {
    pub name: String,
    pub profile: PresetProfileConfig,
}

impl Default for CustomPresetConfig {
    fn default() -> Self {
        Self {
            name: "Custom preset".to_string(),
            profile: PresetProfileConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PresetProfileConfig {
    pub performance_mode: String,
    pub reference_mode: String,
    pub mini_sectors: u16,
    pub style: StyleConfig,
    pub units: UnitsConfig,
    pub coaching_config: CoachingConfig,
    pub layout: LayoutConfig,
    pub extra_widgets: BTreeMap<String, WidgetInstanceConfig>,
    pub title: bool,
    pub speed_gear_rpm: bool,
    pub pedals: bool,
    pub steering: bool,
    pub lap_info: bool,
    pub lap_timing: bool,
    pub sectors: bool,
    pub mini_sector_widget: bool,
    pub input_history: bool,
    pub delta_timing: bool,
    pub ghost_inputs: bool,
    pub coaching: bool,
    pub performance_monitor: bool,
}

impl PresetProfileConfig {
    fn with_extra_widgets(mut self, ids: &[&str]) -> Self {
        for id in ids {
            if let Some(widget) = self.extra_widgets.get_mut(*id) {
                widget.enabled = true;
            }
        }
        self
    }

    fn base() -> Self {
        Self {
            performance_mode: "normal".to_string(),
            reference_mode: "personal_best".to_string(),
            mini_sectors: 40,
            style: StyleConfig::default(),
            units: UnitsConfig::default(),
            coaching_config: CoachingConfig::default(),
            layout: LayoutConfig::default(),
            extra_widgets: crate::widgets::WIDGET_CATALOG
                .iter()
                .enumerate()
                .map(|(index, definition)| {
                    (
                        definition.id.to_string(),
                        WidgetInstanceConfig::disabled(index),
                    )
                })
                .collect(),
            title: false,
            speed_gear_rpm: false,
            pedals: false,
            steering: false,
            lap_info: false,
            lap_timing: false,
            sectors: false,
            mini_sector_widget: false,
            input_history: false,
            delta_timing: false,
            ghost_inputs: false,
            coaching: false,
            performance_monitor: false,
        }
    }

    fn practice() -> Self {
        Self {
            performance_mode: "normal".to_string(),
            reference_mode: "last_lap".to_string(),
            mini_sectors: 40,
            title: true,
            speed_gear_rpm: true,
            pedals: true,
            steering: true,
            lap_info: true,
            lap_timing: true,
            sectors: true,
            mini_sector_widget: true,
            input_history: true,
            delta_timing: true,
            ghost_inputs: true,
            coaching: true,
            performance_monitor: false,
            ..Self::base()
        }
    }

    fn qualifying() -> Self {
        Self {
            performance_mode: "high_refresh".to_string(),
            reference_mode: "personal_best".to_string(),
            mini_sectors: 60,
            title: true,
            speed_gear_rpm: true,
            pedals: true,
            steering: true,
            lap_info: true,
            lap_timing: true,
            sectors: true,
            mini_sector_widget: true,
            input_history: true,
            delta_timing: true,
            ghost_inputs: true,
            coaching: true,
            performance_monitor: false,
            ..Self::base()
        }
    }

    fn race() -> Self {
        Self {
            performance_mode: "eco".to_string(),
            reference_mode: "session_best".to_string(),
            mini_sectors: 20,
            title: false,
            speed_gear_rpm: true,
            pedals: true,
            steering: false,
            lap_info: true,
            lap_timing: true,
            sectors: true,
            mini_sector_widget: true,
            input_history: false,
            delta_timing: true,
            ghost_inputs: false,
            coaching: false,
            performance_monitor: false,
            ..Self::base()
        }
    }

    fn endurance() -> Self {
        Self {
            performance_mode: "eco".to_string(),
            reference_mode: "session_best".to_string(),
            mini_sectors: 20,
            title: false,
            speed_gear_rpm: true,
            pedals: false,
            steering: false,
            lap_info: true,
            lap_timing: true,
            sectors: true,
            mini_sector_widget: false,
            input_history: false,
            delta_timing: true,
            ghost_inputs: false,
            coaching: false,
            performance_monitor: false,
            ..Self::base()
        }
    }

    fn minimal() -> Self {
        Self {
            performance_mode: "normal".to_string(),
            reference_mode: "personal_best".to_string(),
            mini_sectors: 20,
            title: false,
            speed_gear_rpm: true,
            pedals: true,
            steering: false,
            lap_info: false,
            lap_timing: false,
            sectors: false,
            mini_sector_widget: false,
            input_history: false,
            delta_timing: true,
            ghost_inputs: false,
            coaching: false,
            performance_monitor: false,
            ..Self::base()
        }
    }

    fn normalize(&mut self) {
        self.mini_sectors = self.mini_sectors.clamp(1, 200);
        self.style.opacity = self.style.opacity.clamp(32, 255);
        self.style.scale = self.style.scale.clamp(0.65, 1.75);
        self.layout.normalize();
        self.units.normalize();
        self.coaching_config.normalize();
        for widget in self.extra_widgets.values_mut() {
            widget.layout.normalize();
            widget.style.normalize();
            widget.options.normalize();
        }
    }
}

impl Default for PresetProfileConfig {
    fn default() -> Self {
        Self::practice()
    }
}

fn arrange_preset_profile_layout(profile: &mut PresetProfileConfig) {
    let left = 14;
    let width = 392;
    let gap = 10;
    let mut y = 10;
    let mut z = 10;

    if preset_surface_enabled(profile, "telemetry") {
        place_profile_layout(&mut profile.layout, "telemetry", left, y, width, 52, z);
        y += 52 + gap;
        z += 10;
    }
    let top_row = enabled_profile_row(profile, &[("inputs", 96), ("lap_timing", 44)]);
    place_profile_row(
        &mut profile.layout,
        &top_row,
        left,
        width,
        gap,
        &mut y,
        &mut z,
    );
    let middle_row = enabled_profile_row(profile, &[("timing", 54), ("sectors", 44)]);
    place_profile_row(
        &mut profile.layout,
        &middle_row,
        left,
        width,
        gap,
        &mut y,
        &mut z,
    );
    let lower_row = enabled_profile_row(profile, &[("mini_sectors", 44), ("performance", 28)]);
    place_profile_row(
        &mut profile.layout,
        &lower_row,
        left,
        width,
        gap,
        &mut y,
        &mut z,
    );
    if preset_surface_enabled(profile, "coaching") {
        place_profile_layout(&mut profile.layout, "coaching", left, y, width, 58, z);
        y += 58 + gap;
        z += 10;
    }
    place_profile_extra_widgets(&mut profile.extra_widgets, y, z);
}

fn place_profile_row(
    layout: &mut LayoutConfig,
    items: &[(&str, i32)],
    left: i32,
    width: i32,
    gap: i32,
    y: &mut i32,
    z: &mut i32,
) {
    if items.is_empty() {
        return;
    }
    if items.len() == 1 {
        let (key, height) = items[0];
        place_profile_layout(layout, key, left, *y, width, height, *z);
        *y += height + gap;
        *z += 10;
        return;
    }

    let column_width = (width - gap) / 2;
    let row_height = items.iter().map(|(_, height)| *height).max().unwrap_or(44);
    for (index, (key, height)) in items.iter().copied().enumerate() {
        place_profile_layout(
            layout,
            key,
            left + index as i32 * (column_width + gap),
            *y,
            column_width,
            height,
            *z,
        );
        *z += 10;
    }
    *y += row_height + gap;
}

fn enabled_profile_row<'a>(
    profile: &PresetProfileConfig,
    candidates: &'a [(&'a str, i32)],
) -> Vec<(&'a str, i32)> {
    candidates
        .iter()
        .copied()
        .filter(|(key, _)| preset_surface_enabled(profile, key))
        .collect()
}

fn place_profile_layout(
    layout: &mut LayoutConfig,
    key: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    z_index: i32,
) {
    let target = match key {
        "telemetry" => &mut layout.telemetry,
        "inputs" => &mut layout.inputs,
        "lap_timing" => &mut layout.lap_timing,
        "timing" => &mut layout.timing,
        "sectors" => &mut layout.sectors,
        "mini_sectors" => &mut layout.mini_sectors,
        "coaching" => &mut layout.coaching,
        "performance" => &mut layout.performance,
        _ => return,
    };
    target.x = x;
    target.y = y;
    target.width = width;
    target.height = height;
    target.z_index = z_index;
}

fn place_profile_extra_widgets(
    extra_widgets: &mut BTreeMap<String, WidgetInstanceConfig>,
    top: i32,
    start_z: i32,
) {
    let mut enabled = extra_widgets
        .iter()
        .filter(|(_, widget)| widget.enabled)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    enabled.sort_by_key(|id| (id != "standings", id.clone()));

    let left = 14;
    let column_width = 196;
    let row_height = 52;
    let gap = 10;
    let mut slot = 0;
    for id in enabled {
        let Some(widget) = extra_widgets.get_mut(&id) else {
            continue;
        };
        if id == "standings" {
            widget.layout.x = left;
            widget.layout.y = top;
            widget.layout.width = 392;
            widget.layout.height = 120;
            widget.layout.z_index = start_z + slot;
            slot += 4;
            continue;
        }
        let column = slot % 2;
        let row = slot / 2;
        widget.layout.x = left + column * (column_width + gap);
        widget.layout.y = top + row * (row_height + gap);
        widget.layout.width = column_width;
        widget.layout.height = row_height;
        widget.layout.z_index = start_z + slot;
        slot += 1;
    }
}

fn preset_surface_enabled(profile: &PresetProfileConfig, surface: &str) -> bool {
    match surface {
        "telemetry" => profile.title || profile.speed_gear_rpm,
        "inputs" => profile.pedals || profile.steering || profile.input_history,
        "lap_timing" => profile.lap_info || profile.lap_timing,
        "timing" => profile.delta_timing,
        "sectors" => profile.sectors,
        "mini_sectors" => profile.mini_sector_widget,
        "coaching" => profile.coaching && profile.coaching_config.mode != "off",
        "performance" => profile.performance_monitor,
        _ => false,
    }
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            reference_mode: "personal_best".to_string(),
            mini_sectors: 40,
            brake_threshold: 0.10,
            throttle_threshold: 0.10,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Toml(toml::de::Error),
    TomlSer(toml::ser::Error),
    Conflict {
        expected: u64,
        actual: u64,
    },
    UnsupportedVersion {
        found: u32,
        supported: u32,
    },
    UnknownOverlayLayer(String),
    InvalidHotkey {
        field: &'static str,
        binding: String,
    },
    DuplicateHotkey {
        first_field: &'static str,
        second_field: &'static str,
        binding: String,
    },
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(error: toml::ser::Error) -> Self {
        Self::TomlSer(error)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read overlay config: {error}"),
            Self::Toml(error) => write!(f, "overlay config has invalid TOML: {error}"),
            Self::TomlSer(error) => write!(f, "could not write overlay config: {error}"),
            Self::Conflict { expected, actual } => write!(
                f,
                "overlay config changed while editing (expected revision {expected}, found {actual})"
            ),
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "overlay config version {found} is newer than supported version {supported}"
            ),
            Self::UnknownOverlayLayer(id) => write!(f, "overlay layer '{id}' does not exist"),
            Self::InvalidHotkey { field, binding } => write!(
                f,
                "hotkey '{field}' must use F1 through F12 with optional Ctrl, Shift, or Alt modifiers (found '{binding}')"
            ),
            Self::DuplicateHotkey {
                first_field,
                second_field,
                binding,
            } => write!(
                f,
                "hotkey '{binding}' is assigned to both '{first_field}' and '{second_field}'"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn default_config_text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let mut config: OverlayConfig = toml::from_str(DEFAULT_CONFIG_TEMPLATE)
            .expect("the built-in HashOverlay configuration must be valid TOML");
        config.presets = PresetConfig::default();
        let race = config.presets.race.clone();
        config.window.height = 450;
        config.performance.mode = race.performance_mode.clone();
        config.style = race.style.clone();
        config.units = race.units.clone();
        config.coaching = race.coaching_config.clone();
        config.layout = race.layout.clone();
        config.timing.reference_mode = race.reference_mode.clone();
        config.timing.mini_sectors = race.mini_sectors;
        config.widgets = WidgetConfig {
            title: race.title,
            speed_gear_rpm: race.speed_gear_rpm,
            pedals: race.pedals,
            steering: race.steering,
            lap_info: race.lap_info,
            lap_timing: race.lap_timing,
            sectors: false,
            mini_sector_widget: false,
            input_history: race.input_history,
            delta_timing: race.delta_timing,
            ghost_inputs: race.ghost_inputs,
            coaching: race.coaching,
            performance_monitor: race.performance_monitor,
        };
        config.extra_widgets = race.extra_widgets;
        if let Some(standings) = config.extra_widgets.get_mut("standings") {
            standings.enabled = false;
        }
        toml::to_string_pretty(&config)
            .expect("the built-in HashOverlay configuration must serialize")
    })
}

const DEFAULT_CONFIG_TEMPLATE: &str = r##"# HashOverlay configuration
# Open with: hashoverlay --configure

config_version = 7

[window]
x = 40
y = 40
width = 420
height = 230
refresh_hz = 60
sample_ms = 10
history_samples = 180

[style]
opacity = 230
scale = 1.0
line_thickness = 2
background = "#202020"
border = "#666666"
primary_text = "#ffffff"
secondary_text = "#d0d0d0"
throttle = "#44dd22"
brake = "#ee4422"
clutch = "#22dddd"
steering = "#eeeeee"
delta_gain = "#44dd22"
delta_loss = "#ee4422"
delta_neutral = "#f2bc57"
reference = "#aaaaaa"
rpm = "#f2bc57"
coaching_warning = "#f2bc57"
coaching_positive = "#44dd22"
font_family = "Segoe UI"
font_size = 14
font_weight = 500
large_number_size = 24
theme = "hashoverlay_default"

[widgets]
title = true
speed_gear_rpm = true
pedals = true
steering = true
lap_info = true
lap_timing = true
sectors = true
mini_sector_widget = true
input_history = true
delta_timing = true
ghost_inputs = true
coaching = true
performance_monitor = false

[layout]
lock_all = false
snap_to_edges = true
snap_to_grid = false
snap_to_widgets = true
grid_size = 10
snap_distance = 12

[layout.telemetry]
x = 14
y = 10
width = 392
height = 52
locked = false
scale = 1.0
opacity = 1.0
z_index = 10

[layout.inputs]
x = 14
y = 64
width = 240
height = 106
locked = false
scale = 1.0
opacity = 1.0
z_index = 20

[layout.lap_timing]
x = 170
y = 64
width = 236
height = 42
locked = false
scale = 1.0
opacity = 1.0
z_index = 30

[layout.timing]
x = 170
y = 108
width = 236
height = 66
locked = false
scale = 1.0
opacity = 1.0
z_index = 40

[layout.sectors]
x = 14
y = 174
width = 190
height = 42
locked = false
scale = 1.0
opacity = 1.0
z_index = 50

[layout.mini_sectors]
x = 210
y = 174
width = 196
height = 42
locked = false
scale = 1.0
opacity = 1.0
z_index = 60

[layout.coaching]
x = 260
y = 48
width = 146
height = 58
locked = false
scale = 1.0
opacity = 1.0
z_index = 70

[layout.performance]
x = 14
y = 170
width = 392
height = 20
locked = false
scale = 1.0
opacity = 1.0
z_index = 80

[units]
speed = "kmh"
temperature = "celsius"
pressure = "kpa"
fuel = "liters"

[coaching]
mode = "practice"
brake_timing = true
throttle_timing = true
input_match = true
speed = true
gear = true
speed_threshold_kph = 5.0
timing_deadband_m = 3.0
event_match_tolerance_m = 80.0
max_hints = 4

[timing]
reference_mode = "personal_best"
mini_sectors = 40
brake_threshold = 0.10
throttle_threshold = 0.10

[hotkeys]
toggle_overlay = "F9"
edit_mode = "F10"
toggle_coaching = "Shift+F10"
cycle_preset = "Ctrl+Shift+F9"

[performance]
# eco = 50 Hz telemetry / 30 FPS render
# normal = 100 Hz telemetry / 60 FPS render
# high_refresh = 100 Hz telemetry / 120 FPS render
# custom = keep refresh_hz and sample_ms from [window]
mode = "normal"

[presets]
custom = []

[presets.practice]
performance_mode = "normal"
reference_mode = "last_lap"
mini_sectors = 40
title = true
speed_gear_rpm = true
pedals = true
steering = true
lap_info = true
lap_timing = true
sectors = true
mini_sector_widget = true
input_history = true
delta_timing = true
ghost_inputs = true
coaching = true
performance_monitor = false

[presets.qualifying]
performance_mode = "high_refresh"
reference_mode = "personal_best"
mini_sectors = 60
title = true
speed_gear_rpm = true
pedals = true
steering = true
lap_info = true
lap_timing = true
sectors = true
mini_sector_widget = true
input_history = true
delta_timing = true
ghost_inputs = true
coaching = true
performance_monitor = false

[presets.race]
performance_mode = "eco"
reference_mode = "session_best"
mini_sectors = 20
title = false
speed_gear_rpm = true
pedals = true
steering = false
lap_info = true
lap_timing = true
sectors = true
mini_sector_widget = true
input_history = false
delta_timing = true
ghost_inputs = false
coaching = false
performance_monitor = false
"##;

pub fn parse_color(value: &str, fallback: u32) -> u32 {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return fallback;
    }

    let Ok(rgb) = u32::from_str_radix(trimmed, 16) else {
        return fallback;
    };
    let red = (rgb >> 16) & 0xff;
    let green = (rgb >> 8) & 0xff;
    let blue = rgb & 0xff;

    red | (green << 8) | (blue << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_toml() {
        let config: OverlayConfig = toml::from_str(default_config_text()).unwrap();

        assert_eq!(config.config_version, 7);
        assert_eq!(config.window.width, 420);
        assert_eq!(config.window.height, 450);
        assert!(!config.widgets.input_history);
        assert_eq!(config.timing.mini_sectors, 20);
        assert_eq!(config.hotkeys.toggle_overlay, "F9");
        assert_eq!(config.hotkeys.toggle_coaching, "Shift+F10");
        assert_eq!(config.performance.mode, "eco");
        assert_eq!(config.style.scale, 1.0);
        assert_eq!(config.style.line_thickness, 2);
        assert_eq!(config.style.font_size, 14);
        assert_eq!(config.units.speed, "kmh");
        assert_eq!(config.units.temperature, "celsius");
        assert_eq!(config.units.pressure, "kpa");
        assert_eq!(config.units.fuel, "liters");
        assert_eq!(config.coaching.mode, "practice");
        assert!(config.coaching.brake_timing);
        assert!(config.layout.snap_to_edges);
        assert_eq!(config.layout.grid_size, 10);
        assert_eq!(config.layout.telemetry.z_index, 10);
        assert_eq!(config.layout.telemetry.opacity, 1.0);
        assert_eq!(config.layout.inputs.width, 191);
        assert!(config.extra_widgets["relative"].enabled);
        assert!(config.extra_widgets["fuel"].enabled);
        assert!(config.extra_widgets["flags"].enabled);
        assert!(!config.extra_widgets["standings"].enabled);
        assert!(config.presets.custom.is_empty());
        assert_eq!(config.presets.qualifying.performance_mode, "high_refresh");
        assert!(config.presets.race.extra_widgets["relative"].enabled);
        assert!(config.presets.endurance.extra_widgets["damage"].enabled);
        assert_eq!(config.extra_widgets["rpm"].options.shift_start_percent, 70);
        assert_eq!(
            config.extra_widgets["rpm"].options.shift_warning_percent,
            85
        );
        assert_eq!(config.extra_widgets["rpm"].options.limiter_percent, 95);
        assert_eq!(config.extra_widgets["rpm"].options.shift_segments, 10);
    }

    #[test]
    fn parses_web_hex_color_to_gdi_colorref() {
        assert_eq!(parse_color("#112233", 0), 0x00332211);
        assert_eq!(parse_color("bad", 0x00ffffff), 0x00ffffff);
    }

    #[test]
    fn normalizes_risky_values() {
        let mut config = OverlayConfig {
            config_version: 0,
            window: WindowConfig {
                width: 1,
                height: 9999,
                refresh_hz: 1,
                sample_ms: 1,
                history_samples: 1,
                ..WindowConfig::default()
            },
            style: StyleConfig {
                opacity: 1,
                scale: 10.0,
                line_thickness: 99,
                font_size: 99,
                ..StyleConfig::default()
            },
            widgets: WidgetConfig::default(),
            extra_widgets: BTreeMap::new(),
            units: UnitsConfig {
                speed: "knots".to_string(),
                temperature: "rankine".to_string(),
                pressure: "bar".to_string(),
                fuel: "cups".to_string(),
            },
            coaching: CoachingConfig {
                mode: "wild".to_string(),
                speed_threshold_kph: 999.0,
                timing_deadband_m: 999.0,
                max_hints: 99,
                ..CoachingConfig::default()
            },
            layout: LayoutConfig {
                snap_distance: 99,
                grid_size: 7,
                telemetry: WidgetLayout {
                    width: 1,
                    height: 1,
                    scale: 9.0,
                    opacity: 0.0,
                    z_index: 9_999,
                    ..WidgetLayout::default()
                },
                ..LayoutConfig::default()
            },
            timing: TimingConfig::default(),
            hotkeys: HotkeyConfig::default(),
            performance: PerformanceConfig {
                mode: "custom".to_string(),
            },
            presets: PresetConfig::default(),
            overlays: Vec::new(),
        };

        config.normalize();

        assert_eq!(config.config_version, 7);
        assert_eq!(config.window.width, 280);
        assert_eq!(config.window.height, 800);
        assert_eq!(config.window.refresh_hz, 15);
        assert_eq!(config.window.sample_ms, 5);
        assert_eq!(config.window.history_samples, 16);
        assert_eq!(config.style.opacity, 32);
        assert_eq!(config.style.scale, 1.75);
        assert_eq!(config.style.line_thickness, 8);
        assert_eq!(config.style.font_size, 36);
        assert_eq!(config.units.speed, "kmh");
        assert_eq!(config.units.temperature, "celsius");
        assert_eq!(config.units.pressure, "kpa");
        assert_eq!(config.units.fuel, "liters");
        assert_eq!(config.coaching.mode, "practice");
        assert_eq!(config.coaching.speed_threshold_kph, 40.0);
        assert_eq!(config.coaching.timing_deadband_m, 50.0);
        assert_eq!(config.coaching.event_match_tolerance_m, 80.0);
        assert_eq!(config.coaching.max_hints, 6);
        assert_eq!(config.layout.snap_distance, 64);
        assert_eq!(config.layout.grid_size, 10);
        assert_eq!(config.layout.telemetry.width, 48);
        assert_eq!(config.layout.telemetry.height, 20);
        assert_eq!(config.layout.telemetry.scale, 2.0);
        assert_eq!(config.layout.telemetry.opacity, 0.1);
        assert_eq!(config.layout.telemetry.z_index, 1000);
        assert_eq!(config.timing.mini_sectors, 40);
        let rpm_options = &config.extra_widgets["rpm"].options;
        assert_eq!(rpm_options.shift_start_percent, 70);
        assert_eq!(rpm_options.shift_warning_percent, 85);
        assert_eq!(rpm_options.limiter_percent, 95);
        assert_eq!(rpm_options.shift_segments, 10);
    }

    #[test]
    fn migrates_v3_configs_to_explicit_display_units() {
        let mut config = OverlayConfig {
            config_version: 3,
            units: UnitsConfig {
                speed: "mph".to_string(),
                temperature: String::new(),
                pressure: String::new(),
                fuel: String::new(),
            },
            ..OverlayConfig::default()
        };

        migrate_config(&mut config, 3);
        config.normalize();

        assert_eq!(config.config_version, CURRENT_CONFIG_VERSION);
        assert_eq!(config.units.speed, "mph");
        assert_eq!(config.units.temperature, "celsius");
        assert_eq!(config.units.pressure, "kpa");
        assert_eq!(config.units.fuel, "liters");
    }

    #[test]
    fn historical_migrations_advance_one_schema_version_at_a_time() {
        let mut config = OverlayConfig {
            config_version: 5,
            ..OverlayConfig::default()
        };

        migrate_v5_to_v6(&mut config);

        assert_eq!(config.config_version, 6);
        migrate_v6_to_v7(&mut config);
        assert_eq!(config.config_version, CURRENT_CONFIG_VERSION);
    }

    #[test]
    fn fills_missing_widget_instances_without_preserving_unknown_ids() {
        let mut config = OverlayConfig::default();
        config.extra_widgets.insert(
            "unknown-widget".to_string(),
            WidgetInstanceConfig::default(),
        );

        config.normalize();

        assert!(!config.extra_widgets.contains_key("unknown-widget"));
        assert!(config.extra_widgets.contains_key("fuel"));
        assert!(config.extra_widgets.contains_key("weather"));
    }

    #[test]
    fn creates_and_filters_independent_overlay_layers() {
        let mut config = OverlayConfig::default();
        config.normalize();
        config.overlays.push(OverlayLayerConfig {
            id: "coach".to_string(),
            name: "Coach panel".to_string(),
            enabled: true,
            widgets: vec!["coaching".to_string()],
            ..OverlayLayerConfig::default()
        });
        config.normalize();

        let coach = config.for_overlay_layer(Some("coach")).unwrap();
        assert_eq!(coach.window.width, WindowConfig::default().width);
        assert!(coach.widgets.coaching);
        assert!(!coach.widgets.speed_gear_rpm);
        assert!(!coach.extra_widgets["fuel"].enabled);
        assert!(config.overlays.iter().any(|layer| layer.id == "main"));
    }

    #[test]
    fn preserves_performance_surface_membership() {
        let mut config = OverlayConfig::default();
        config.widgets.performance_monitor = true;
        config.overlays = vec![OverlayLayerConfig {
            widgets: vec!["performance".to_string()],
            ..OverlayLayerConfig::default()
        }];

        config.normalize();

        assert_eq!(config.overlays[0].widgets, vec!["performance"]);
        assert!(
            config
                .for_overlay_layer(Some("main"))
                .unwrap()
                .widgets
                .performance_monitor
        );
    }

    #[test]
    fn rejects_unknown_explicit_overlay_layer() {
        let mut config = OverlayConfig::default();
        config.normalize();

        assert!(matches!(
            config.for_overlay_layer(Some("typo")),
            Err(ConfigError::UnknownOverlayLayer(id)) if id == "typo"
        ));
        assert!(config.for_overlay_layer(None).is_ok());
    }

    #[test]
    fn rejects_future_config_without_rewriting_it() {
        let temp_path = std::env::temp_dir().join(format!(
            "hashoverlay-future-config-{}.toml",
            std::process::id()
        ));
        let config = OverlayConfig {
            config_version: 999,
            ..OverlayConfig::default()
        };
        let original = toml::to_string_pretty(&config).unwrap();
        fs::write(&temp_path, &original).unwrap();

        assert!(matches!(
            OverlayConfig::load(&temp_path),
            Err(ConfigError::UnsupportedVersion {
                found: 999,
                supported: CURRENT_CONFIG_VERSION
            })
        ));
        assert_eq!(fs::read_to_string(&temp_path).unwrap(), original);
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn rejects_future_config_before_save_or_revision_check() {
        let temp_path = std::env::temp_dir().join(format!(
            "hashoverlay-future-save-{}.toml",
            std::process::id()
        ));
        let mut current = OverlayConfig::default();
        current.save(&temp_path).unwrap();
        let original = fs::read_to_string(&temp_path).unwrap();
        let revision = OverlayConfig::revision(&temp_path).unwrap();
        let mut future = OverlayConfig {
            config_version: 999,
            ..OverlayConfig::default()
        };

        assert!(matches!(
            future.save(&temp_path),
            Err(ConfigError::UnsupportedVersion { found: 999, .. })
        ));
        assert!(matches!(
            future.save_if_revision(&temp_path, revision),
            Err(ConfigError::UnsupportedVersion { found: 999, .. })
        ));
        assert_eq!(fs::read_to_string(&temp_path).unwrap(), original);
        assert_eq!(OverlayConfig::revision(&temp_path).unwrap(), revision);
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn rejects_stale_config_revision() {
        let temp_path = std::env::temp_dir().join(format!(
            "hashoverlay-config-revision-{}.toml",
            std::process::id()
        ));
        let mut config = OverlayConfig::default();
        config.save(&temp_path).unwrap();
        let revision = OverlayConfig::revision(&temp_path).unwrap();
        fs::write(&temp_path, "config_version = 7\n").unwrap();

        assert!(matches!(
            config.save_if_revision(&temp_path, revision),
            Err(ConfigError::Conflict { .. })
        ));
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn rejects_invalid_or_duplicate_hotkeys_before_saving() {
        let mut invalid = OverlayConfig::default();
        invalid.hotkeys.toggle_overlay = "Ctrl+K".to_string();
        assert!(matches!(
            invalid.validate_for_save(),
            Err(ConfigError::InvalidHotkey {
                field: "toggle_overlay",
                ..
            })
        ));

        let mut duplicate = OverlayConfig::default();
        duplicate.hotkeys.edit_mode = " f09 ".to_string();
        assert!(matches!(
            duplicate.validate_for_save(),
            Err(ConfigError::DuplicateHotkey {
                first_field: "toggle_overlay",
                second_field: "edit_mode",
                ..
            })
        ));
    }

    #[test]
    fn saves_when_revision_matches_and_returns_new_revision() {
        let temp_path = std::env::temp_dir().join(format!(
            "hashoverlay-config-revision-success-{}.toml",
            std::process::id()
        ));
        let mut config = OverlayConfig::default();
        config.save(&temp_path).unwrap();
        let revision = OverlayConfig::revision(&temp_path).unwrap();

        let mut updated = config.clone();
        updated.style.theme = "high_contrast".to_string();
        let new_revision = updated.save_if_revision(&temp_path, revision).unwrap();

        assert_ne!(new_revision, revision);
        assert_eq!(
            OverlayConfig::load(&temp_path).unwrap().style.theme,
            "high_contrast"
        );
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn loading_v7_without_overlay_layers_persists_normalized_layers() {
        let temp_path = std::env::temp_dir().join(format!(
            "hashoverlay-missing-overlays-{}.toml",
            std::process::id()
        ));
        let mut config = OverlayConfig {
            config_version: CURRENT_CONFIG_VERSION,
            ..OverlayConfig::default()
        };
        config.overlays.clear();
        fs::write(&temp_path, toml::to_string_pretty(&config).unwrap()).unwrap();

        let loaded = OverlayConfig::load(&temp_path).unwrap();
        let persisted = fs::read_to_string(&temp_path).unwrap();
        let _ = fs::remove_file(&temp_path);

        assert_eq!(loaded.overlays.len(), 1);
        assert!(persisted.contains("[[overlays]]"));
    }

    #[test]
    fn preset_profiles_capture_complete_overlay_state() {
        let profile = PresetProfileConfig::practice();

        assert_eq!(profile.style.theme, "hashoverlay_default");
        assert_eq!(profile.units.speed, "kmh");
        assert!(profile.coaching_config.brake_timing);
        assert!(profile.extra_widgets.contains_key("fuel"));
        assert_eq!(profile.layout.telemetry.z_index, 10);
    }

    #[test]
    fn shipped_presets_enable_their_real_widget_sets() {
        let presets = PresetConfig::default();
        assert!(presets.race.extra_widgets["relative"].enabled);
        assert!(presets.race.extra_widgets["standings"].enabled);
        assert!(presets.race.extra_widgets["fuel"].enabled);
        assert!(presets.endurance.extra_widgets["energy"].enabled);
        assert!(presets.practice.extra_widgets["tyres"].enabled);
        assert!(!presets.minimal.extra_widgets["relative"].enabled);
    }

    #[test]
    fn cycling_a_preset_updates_the_complete_runtime_profile() {
        let mut config = OverlayConfig::default();
        config.performance.mode = "normal".to_string();
        config.timing.reference_mode = "last_lap".to_string();

        config.cycle_preset();

        assert_eq!(config.performance.mode, "high_refresh");
        assert_eq!(config.timing.reference_mode, "personal_best");
        assert_eq!(
            config.widgets.performance_monitor,
            config.presets.qualifying.performance_monitor
        );
    }

    #[test]
    fn shipped_presets_do_not_overlap_enabled_widgets() {
        let presets = PresetConfig::default();
        for (name, profile) in [
            ("practice", &presets.practice),
            ("qualifying", &presets.qualifying),
            ("race", &presets.race),
            ("endurance", &presets.endurance),
            ("minimal", &presets.minimal),
        ] {
            let areas = enabled_profile_areas(profile);
            for (left_index, left) in areas.iter().enumerate() {
                for right in areas.iter().skip(left_index + 1) {
                    assert!(
                        !areas_overlap(left, right),
                        "{name} preset overlaps {} and {}",
                        left.id,
                        right.id
                    );
                }
            }
        }
    }

    struct TestArea {
        id: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    fn enabled_profile_areas(profile: &PresetProfileConfig) -> Vec<TestArea> {
        let mut areas = [
            ("telemetry", &profile.layout.telemetry),
            ("inputs", &profile.layout.inputs),
            ("lap_timing", &profile.layout.lap_timing),
            ("timing", &profile.layout.timing),
            ("sectors", &profile.layout.sectors),
            ("mini_sectors", &profile.layout.mini_sectors),
            ("coaching", &profile.layout.coaching),
            ("performance", &profile.layout.performance),
        ]
        .into_iter()
        .filter(|(id, _)| preset_surface_enabled(profile, id))
        .map(|(id, layout)| test_area(id, layout))
        .collect::<Vec<_>>();
        areas.extend(
            profile
                .extra_widgets
                .iter()
                .filter(|(_, widget)| widget.enabled)
                .map(|(id, widget)| test_area(id, &widget.layout)),
        );
        areas
    }

    fn test_area(id: &str, layout: &WidgetLayout) -> TestArea {
        TestArea {
            id: id.to_string(),
            x: layout.x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
        }
    }

    fn areas_overlap(left: &TestArea, right: &TestArea) -> bool {
        left.x < right.x + right.width
            && left.x + left.width > right.x
            && left.y < right.y + right.height
            && left.y + left.height > right.y
    }
}
