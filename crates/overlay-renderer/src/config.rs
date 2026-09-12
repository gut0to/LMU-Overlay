use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

use serde::{Deserialize, Serialize};

const CURRENT_CONFIG_VERSION: u32 = 3;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayConfig {
    pub config_version: u32,
    pub window: WindowConfig,
    pub style: StyleConfig,
    pub widgets: WidgetConfig,
    pub layout: LayoutConfig,
    pub units: UnitsConfig,
    pub coaching: CoachingConfig,
    pub timing: TimingConfig,
    pub hotkeys: HotkeyConfig,
    pub performance: PerformanceConfig,
    pub presets: PresetConfig,
}

impl OverlayConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&text)?;
        let needs_migration = config.config_version < CURRENT_CONFIG_VERSION;
        if needs_migration {
            write_migration_backup(path, &text)?;
            config.config_version = CURRENT_CONFIG_VERSION;
        }
        config.normalize();
        if needs_migration {
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
        self.normalize();
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn normalize(&mut self) {
        if self.config_version == 0 || self.config_version > CURRENT_CONFIG_VERSION {
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
        self.style.font_size = self.style.font_size.clamp(8, 36);
        self.style.font_weight = self.style.font_weight.clamp(100, 900);
        self.style.large_number_size = self.style.large_number_size.clamp(12, 72);
        self.layout.normalize();
        self.units.normalize();
        self.coaching.normalize();
        self.timing.mini_sectors = self.timing.mini_sectors.clamp(1, 200);
        self.timing.brake_threshold = self.timing.brake_threshold.clamp(0.01, 1.0);
        self.timing.throttle_threshold = self.timing.throttle_threshold.clamp(0.01, 1.0);
        self.presets.normalize();
    }
}

fn write_migration_backup(path: &Path, text: &str) -> io::Result<()> {
    let backup_path = path.with_extension("toml.v2.bak");
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
    temp_path.set_extension(extension);
    temp_path
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
}

impl UnitsConfig {
    fn normalize(&mut self) {
        if !matches!(self.speed.as_str(), "kmh" | "mph") {
            self.speed = "kmh".to_string();
        }
    }
}

impl Default for UnitsConfig {
    fn default() -> Self {
        Self {
            speed: "kmh".to_string(),
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
    pub max_hints: u8,
}

impl CoachingConfig {
    fn normalize(&mut self) {
        if !matches!(self.mode.as_str(), "off" | "race" | "practice" | "attack") {
            self.mode = "practice".to_string();
        }
        self.speed_threshold_kph = self.speed_threshold_kph.clamp(1.0, 40.0);
        self.timing_deadband_m = self.timing_deadband_m.clamp(0.0, 50.0);
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    pub lock_all: bool,
    pub snap_to_edges: bool,
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
            snap_distance: 12,
            telemetry: WidgetLayout {
                x: 14,
                y: 10,
                width: 392,
                height: 52,
                locked: false,
            },
            inputs: WidgetLayout {
                x: 14,
                y: 64,
                width: 240,
                height: 106,
                locked: false,
            },
            lap_timing: WidgetLayout {
                x: 170,
                y: 64,
                width: 236,
                height: 42,
                locked: false,
            },
            timing: WidgetLayout {
                x: 170,
                y: 108,
                width: 236,
                height: 66,
                locked: false,
            },
            sectors: WidgetLayout {
                x: 14,
                y: 174,
                width: 190,
                height: 42,
                locked: false,
            },
            mini_sectors: WidgetLayout {
                x: 210,
                y: 174,
                width: 196,
                height: 42,
                locked: false,
            },
            coaching: WidgetLayout {
                x: 260,
                y: 48,
                width: 146,
                height: 58,
                locked: false,
            },
            performance: WidgetLayout {
                x: 14,
                y: 170,
                width: 392,
                height: 20,
                locked: false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub locked: bool,
}

impl WidgetLayout {
    fn normalize(&mut self) {
        self.x = self.x.clamp(-2000, 8000);
        self.y = self.y.clamp(-2000, 8000);
        self.width = self.width.clamp(48, 1600);
        self.height = self.height.clamp(20, 1000);
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
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle_overlay: "F9".to_string(),
            edit_mode: "F10".to_string(),
        }
    }
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
    pub custom: Vec<CustomPresetConfig>,
}

impl PresetConfig {
    fn normalize(&mut self) {
        self.practice.normalize();
        self.qualifying.normalize();
        self.race.normalize();
        self.custom.truncate(32);
        for preset in &mut self.custom {
            preset.profile.normalize();
        }
    }
}

impl Default for PresetConfig {
    fn default() -> Self {
        Self {
            practice: PresetProfileConfig::practice(),
            qualifying: PresetProfileConfig::qualifying(),
            race: PresetProfileConfig::race(),
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
        }
    }

    fn normalize(&mut self) {
        self.mini_sectors = self.mini_sectors.clamp(1, 200);
    }
}

impl Default for PresetProfileConfig {
    fn default() -> Self {
        Self::practice()
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
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn default_config_text() -> &'static str {
    r##"# HashOverlay configuration
# Open with: hashoverlay --configure

config_version = 3

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
snap_distance = 12

[layout.telemetry]
x = 14
y = 10
width = 392
height = 52
locked = false

[layout.inputs]
x = 14
y = 64
width = 240
height = 106
locked = false

[layout.lap_timing]
x = 170
y = 64
width = 236
height = 42
locked = false

[layout.timing]
x = 170
y = 108
width = 236
height = 66
locked = false

[layout.sectors]
x = 14
y = 174
width = 190
height = 42
locked = false

[layout.mini_sectors]
x = 210
y = 174
width = 196
height = 42
locked = false

[layout.coaching]
x = 260
y = 48
width = 146
height = 58
locked = false

[layout.performance]
x = 14
y = 170
width = 392
height = 20
locked = false

[units]
speed = "kmh"

[coaching]
mode = "practice"
brake_timing = true
throttle_timing = true
input_match = true
speed = true
gear = true
speed_threshold_kph = 5.0
timing_deadband_m = 3.0
max_hints = 4

[timing]
reference_mode = "personal_best"
mini_sectors = 40
brake_threshold = 0.10
throttle_threshold = 0.10

[hotkeys]
toggle_overlay = "F9"
edit_mode = "F10"

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
"##
}

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

        assert_eq!(config.config_version, 3);
        assert_eq!(config.window.width, 420);
        assert!(config.widgets.input_history);
        assert_eq!(config.timing.mini_sectors, 40);
        assert_eq!(config.hotkeys.toggle_overlay, "F9");
        assert_eq!(config.performance.mode, "normal");
        assert_eq!(config.style.scale, 1.0);
        assert_eq!(config.style.line_thickness, 2);
        assert_eq!(config.style.font_size, 14);
        assert_eq!(config.units.speed, "kmh");
        assert_eq!(config.coaching.mode, "practice");
        assert!(config.coaching.brake_timing);
        assert!(config.layout.snap_to_edges);
        assert_eq!(config.layout.inputs.width, 240);
        assert!(config.presets.custom.is_empty());
        assert_eq!(config.presets.qualifying.performance_mode, "high_refresh");
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
            units: UnitsConfig {
                speed: "knots".to_string(),
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
                telemetry: WidgetLayout {
                    width: 1,
                    height: 1,
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
        };

        config.normalize();

        assert_eq!(config.config_version, 3);
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
        assert_eq!(config.coaching.mode, "practice");
        assert_eq!(config.coaching.speed_threshold_kph, 40.0);
        assert_eq!(config.coaching.timing_deadband_m, 50.0);
        assert_eq!(config.coaching.max_hints, 6);
        assert_eq!(config.layout.snap_distance, 64);
        assert_eq!(config.layout.telemetry.width, 48);
        assert_eq!(config.layout.telemetry.height, 20);
        assert_eq!(config.timing.mini_sectors, 40);
    }
}
