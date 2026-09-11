use std::{fs, path::PathBuf};

use overlay_renderer::config::OverlayConfig;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ConfigResponse {
    path: String,
    config: OverlayConfig,
}

#[tauri::command]
fn load_config() -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    OverlayConfig::save_default(&path).map_err(|error| error.to_string())?;
    let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
    Ok(ConfigResponse::new(path, config))
}

#[tauri::command]
fn save_config(mut config: OverlayConfig) -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    config.save(&path).map_err(|error| error.to_string())?;
    let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
    Ok(ConfigResponse::new(path, config))
}

impl ConfigResponse {
    fn new(path: PathBuf, config: OverlayConfig) -> Self {
        Self {
            path: path.display().to_string(),
            config,
        }
    }
}

fn overlay_config_path() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("HashOverlay")
        .join("hashoverlay.toml")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![load_config, save_config])
        .setup(|_| {
            let path = overlay_config_path();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            OverlayConfig::save_default(path)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run HashOverlay settings");
}
