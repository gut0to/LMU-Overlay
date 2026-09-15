use std::{
    fs,
    path::PathBuf,
    process::{Child, Command},
    sync::Mutex,
};

use overlay_renderer::{
    config::{default_config_text, OverlayConfig},
    widget_catalog as overlay_widget_catalog, WidgetDefinition,
};
use serde::Serialize;
use tauri::Manager;

#[derive(Default)]
struct OverlayProcesses(Mutex<Vec<Child>>);

#[derive(Debug, Serialize)]
struct ConfigResponse {
    path: String,
    config: OverlayConfig,
    revision: u64,
}

#[tauri::command]
fn load_config() -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    OverlayConfig::save_default(&path).map_err(|error| error.to_string())?;
    let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
    Ok(ConfigResponse::new(path, config))
}

#[tauri::command]
fn save_config(mut config: OverlayConfig, expected_revision: u64) -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    config
        .save_if_revision(&path, expected_revision)
        .map_err(|error| error.to_string())?;
    let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
    Ok(ConfigResponse::new(path, config))
}

#[tauri::command]
fn default_config() -> Result<OverlayConfig, String> {
    let mut config: OverlayConfig = toml::from_str(default_config_text()).map_err(|error| error.to_string())?;
    config.normalize();
    Ok(config)
}

#[tauri::command]
fn export_config(mut config: OverlayConfig) -> Result<String, String> {
    config.normalize();
    toml::to_string_pretty(&config).map_err(|error| error.to_string())
}

#[tauri::command]
fn import_config(text: String, expected_revision: u64) -> Result<ConfigResponse, String> {
    let mut config: OverlayConfig = toml::from_str(&text).map_err(|error| error.to_string())?;
    let path = overlay_config_path();
    config
        .save_if_revision(&path, expected_revision)
        .map_err(|error| error.to_string())?;
    let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
    Ok(ConfigResponse::new(path, config))
}

#[tauri::command]
fn reset_config() -> Result<ConfigResponse, String> {
    let config: OverlayConfig = toml::from_str(default_config_text()).map_err(|error| error.to_string())?;
    let revision = OverlayConfig::revision(&overlay_config_path()).unwrap_or_default();
    save_config(config, revision)
}

#[tauri::command]
fn widget_catalog() -> Vec<WidgetDefinition> {
    overlay_widget_catalog().to_vec()
}

#[tauri::command]
fn start_overlay(app: tauri::AppHandle, processes: tauri::State<'_, OverlayProcesses>) -> Result<(), String> {
    if host_is_running() {
        return Ok(());
    }
    let mut running = processes
        .0
        .lock()
        .map_err(|_| "Overlay process state is unavailable".to_string())?;
    running.retain_mut(|child| {
        child
            .try_wait()
            .map(|status| status.is_none())
            .unwrap_or(true)
    });
    if !running.is_empty() {
        return Ok(());
    }

    let mut candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("hashoverlay.exe"));
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join("hashoverlay.exe"));
            // `cargo tauri dev` keeps the Settings binary below the workspace
            // while the overlay binary remains in the workspace target folder.
            candidates.push(parent.join("..\\..\\..\\target\\debug\\hashoverlay.exe"));
            candidates.push(parent.join("..\\..\\..\\target\\release\\hashoverlay.exe"));
        }
    }
    let executable = candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "Could not find the bundled HashOverlay executable. Reinstall HashOverlay Settings or build the overlay first.".to_string()
        })?;

    OverlayConfig::load(overlay_config_path())
        .map_err(|error| format!("Could not load overlay configuration: {error}"))?;
    match Command::new(&executable).arg("--overlay").spawn() {
        Ok(child) => running.push(child),
        Err(error) => return Err(format!("Could not start HashOverlay: {error}")),
    }
    Ok(())
}

#[tauri::command]
fn stop_overlay(processes: tauri::State<'_, OverlayProcesses>) -> Result<(), String> {
    let mut running = processes
        .0
        .lock()
        .map_err(|_| "Overlay process state is unavailable".to_string())?;
    stop_running_overlays(&mut running);
    Ok(())
}

fn stop_running_overlays(running: &mut Vec<Child>) {
    for mut child in running.drain(..) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[tauri::command]
fn overlay_status() -> Result<bool, String> {
    Ok(host_is_running())
}

fn host_is_running() -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::Threading::{OpenMutexW, MUTEX_ALL_ACCESS},
        };
        let name: Vec<u16> = "Local\\HashOverlay.Host"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle: HANDLE = OpenMutexW(MUTEX_ALL_ACCESS, 0, name.as_ptr());
        if handle.is_null() {
            return false;
        }
        CloseHandle(handle);
        true
    }

    #[cfg(not(windows))]
    false
}

#[tauri::command]
fn open_config_folder() -> Result<(), String> {
    let folder = overlay_config_path()
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Could not resolve the config folder".to_string())?;
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    Command::new("explorer.exe")
        .arg(folder)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open config folder: {error}"))
}

impl ConfigResponse {
    fn new(path: PathBuf, config: OverlayConfig) -> Self {
        Self {
            path: path.display().to_string(),
            config,
            revision: OverlayConfig::revision(&path).unwrap_or_default(),
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
        .manage(OverlayProcesses::default())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            default_config,
            export_config,
            import_config,
            reset_config,
            widget_catalog,
            start_overlay,
            stop_overlay,
            overlay_status,
            open_config_folder
        ])
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
