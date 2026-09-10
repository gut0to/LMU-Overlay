use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayConfig {
    pub window: WindowConfig,
    pub style: StyleConfig,
    pub widgets: WidgetConfig,
    pub timing: TimingConfig,
}

impl OverlayConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&text)?;
        config.normalize();
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

    fn normalize(&mut self) {
        self.window.width = self.window.width.clamp(280, 1200);
        self.window.height = self.window.height.clamp(140, 800);
        self.window.refresh_hz = self.window.refresh_hz.clamp(15, 144);
        self.window.sample_ms = self.window.sample_ms.clamp(5, 250);
        self.window.history_samples = self.window.history_samples.clamp(16, 900);
        self.style.opacity = self.style.opacity.clamp(32, 255);
        self.timing.mini_sectors = self.timing.mini_sectors.clamp(1, 200);
        self.timing.brake_threshold = self.timing.brake_threshold.clamp(0.01, 1.0);
        self.timing.throttle_threshold = self.timing.throttle_threshold.clamp(0.01, 1.0);
    }
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
            height: 190,
            refresh_hz: 60,
            sample_ms: 10,
            history_samples: 180,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StyleConfig {
    pub opacity: u8,
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
    pub reference: String,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            opacity: 230,
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
            reference: "#aaaaaa".to_string(),
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
    pub input_history: bool,
    pub delta_timing: bool,
    pub ghost_inputs: bool,
    pub coaching: bool,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            title: true,
            speed_gear_rpm: true,
            pedals: true,
            steering: true,
            lap_info: true,
            input_history: true,
            delta_timing: true,
            ghost_inputs: true,
            coaching: true,
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

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read overlay config: {error}"),
            Self::Toml(error) => write!(f, "overlay config has invalid TOML: {error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn default_config_text() -> &'static str {
    r##"# HashOverlay configuration
# Open with: hashoverlay --configure

[window]
x = 40
y = 40
width = 420
height = 190
refresh_hz = 60
sample_ms = 10
history_samples = 180

[style]
opacity = 230
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
reference = "#aaaaaa"

[widgets]
title = true
speed_gear_rpm = true
pedals = true
steering = true
lap_info = true
input_history = true
delta_timing = true
ghost_inputs = true
coaching = true

[timing]
reference_mode = "personal_best"
mini_sectors = 40
brake_threshold = 0.10
throttle_threshold = 0.10
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

        assert_eq!(config.window.width, 420);
        assert!(config.widgets.input_history);
        assert_eq!(config.timing.mini_sectors, 40);
    }

    #[test]
    fn parses_web_hex_color_to_gdi_colorref() {
        assert_eq!(parse_color("#112233", 0), 0x00332211);
        assert_eq!(parse_color("bad", 0x00ffffff), 0x00ffffff);
    }

    #[test]
    fn normalizes_risky_values() {
        let mut config = OverlayConfig {
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
                ..StyleConfig::default()
            },
            widgets: WidgetConfig::default(),
            timing: TimingConfig::default(),
        };

        config.normalize();

        assert_eq!(config.window.width, 280);
        assert_eq!(config.window.height, 800);
        assert_eq!(config.window.refresh_hz, 15);
        assert_eq!(config.window.sample_ms, 5);
        assert_eq!(config.window.history_samples, 16);
        assert_eq!(config.style.opacity, 32);
        assert_eq!(config.timing.mini_sectors, 40);
    }
}
