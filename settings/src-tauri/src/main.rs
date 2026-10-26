mod displays;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command},
    sync::Mutex,
    time::{Duration, Instant},
};

use host_protocol::{Command as HostCommand, Reply as HostReply};

use overlay_renderer::{
    config::{default_config_text, OverlayConfig},
    widget_catalog as overlay_widget_catalog, WidgetDefinition,
};
use serde::Serialize;
use tauri::Manager;

#[derive(Default)]
struct OverlayProcesses(Mutex<Vec<Child>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupDecision {
    ReuseReadyHost,
    Launch,
}

#[derive(Debug, Serialize)]
struct ConfigResponse {
    path: String,
    config: OverlayConfig,
    revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reload_warning: Option<String>,
}

#[tauri::command]
fn load_config() -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    OverlayConfig::save_default(&path).map_err(|error| error.to_string())?;
    ConfigResponse::read(path)
}

#[tauri::command(rename_all = "snake_case")]
async fn save_config(
    config: OverlayConfig,
    expected_revision: String,
) -> Result<ConfigResponse, String> {
    native_task(move || save_config_blocking(config, expected_revision)).await
}

fn save_config_blocking(
    mut config: OverlayConfig,
    expected_revision: String,
) -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    config
        .save_if_revision(&path, parse_revision(&expected_revision)?)
        .map_err(|error| error.to_string())?;
    finish_saved_config(path, reload_overlay_config_after_save())
}

#[tauri::command]
fn default_config() -> Result<OverlayConfig, String> {
    let mut config: OverlayConfig =
        toml::from_str(default_config_text()).map_err(|error| error.to_string())?;
    config.normalize();
    Ok(config)
}

/// Validate a recovery candidate without writing the file or contacting the host.
#[tauri::command]
fn validate_config(mut config: OverlayConfig) -> Result<OverlayConfig, String> {
    config
        .validate_for_save()
        .map_err(|error| error.to_string())?;
    config.normalize();
    Ok(config)
}

#[tauri::command]
fn export_config(mut config: OverlayConfig) -> Result<String, String> {
    config
        .validate_for_save()
        .map_err(|error| error.to_string())?;
    config.normalize();
    toml::to_string_pretty(&config).map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_import(text: String) -> Result<OverlayConfig, String> {
    if text.len() > 4 * 1024 * 1024 {
        return Err(
            "Configuration import exceeds 4 MiB. Choose a smaller TOML configuration.".into(),
        );
    }
    if text.trim().is_empty() {
        return Err("Paste a TOML configuration before previewing the import.".into());
    }
    OverlayConfig::from_import_text(&text).map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "snake_case")]
async fn import_config(text: String, expected_revision: String) -> Result<ConfigResponse, String> {
    native_task(move || import_config_blocking(text, expected_revision)).await
}

fn import_config_blocking(
    text: String,
    expected_revision: String,
) -> Result<ConfigResponse, String> {
    let mut config = preview_import(text)?;
    let path = overlay_config_path();
    config
        .save_if_revision(&path, parse_revision(&expected_revision)?)
        .map_err(|error| error.to_string())?;
    finish_saved_config(path, reload_overlay_config_after_save())
}

#[tauri::command(rename_all = "snake_case")]
async fn reset_config(expected_revision: String) -> Result<ConfigResponse, String> {
    native_task(move || reset_config_blocking(expected_revision)).await
}

fn reset_config_blocking(expected_revision: String) -> Result<ConfigResponse, String> {
    let path = overlay_config_path();
    reset_config_at(&path, parse_revision(&expected_revision)?)?;
    finish_saved_config(path, reload_overlay_config_after_save())
}

fn reset_config_at(path: &std::path::Path, expected_revision: u64) -> Result<(), String> {
    let mut config: OverlayConfig =
        toml::from_str(default_config_text()).map_err(|error| error.to_string())?;
    config
        .save_if_revision(path, expected_revision)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn widget_catalog() -> Vec<WidgetDefinition> {
    overlay_widget_catalog().to_vec()
}

#[tauri::command]
async fn start_overlay(app: tauri::AppHandle) -> Result<(), String> {
    native_task(move || {
        let processes = app.state::<OverlayProcesses>();
        start_overlay_blocking(app.clone(), processes)
    })
    .await
}

fn start_overlay_blocking(
    app: tauri::AppHandle,
    processes: tauri::State<'_, OverlayProcesses>,
) -> Result<(), String> {
    if startup_decision(host_control_is_ready()) == StartupDecision::ReuseReadyHost {
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
        return wait_for_host_startup();
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
    wait_for_host_startup()
}

#[tauri::command]
async fn stop_overlay(app: tauri::AppHandle) -> Result<(), String> {
    native_task(move || stop_overlay_blocking(app.state::<OverlayProcesses>())).await
}

fn stop_overlay_blocking(processes: tauri::State<'_, OverlayProcesses>) -> Result<(), String> {
    let graceful = send_host_command("stop")
        .is_ok_and(|response| HostReply::parse(&response) == Some(HostReply::Stopping));
    if graceful {
        let deadline = Instant::now() + Duration::from_secs(2);
        while host_is_running() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let mut running = processes
        .0
        .lock()
        .map_err(|_| "Overlay process state is unavailable".to_string())?;
    if host_is_running() || !graceful {
        // Emergency fallback only: normal shutdown is sent over the local
        // control pipe so the host can drain its workers and PB writer.
        stop_running_overlays(&mut running);
    } else {
        running.retain_mut(|child| {
            child
                .try_wait()
                .map(|status| status.is_none())
                .unwrap_or(true)
        });
    }
    Ok(())
}

fn stop_running_overlays(running: &mut Vec<Child>) {
    for mut child in running.drain(..) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[tauri::command]
async fn overlay_status() -> Result<bool, String> {
    native_task(move || overlay_status_blocking()).await
}

fn overlay_status_blocking() -> Result<bool, String> {
    Ok(host_control_is_ready())
}

#[tauri::command]
async fn toggle_overlay_edit_mode() -> Result<String, String> {
    native_task(move || toggle_overlay_edit_mode_blocking()).await
}

fn toggle_overlay_edit_mode_blocking() -> Result<String, String> {
    let response = send_host_command("edit")
        .map_err(|error| format!("Could not reach overlay host: {error}"))?;
    if response == "edit_on" || response == "edit_off" {
        Ok(response)
    } else {
        Err(format!("host returned '{response}'"))
    }
}

#[tauri::command]
async fn overlay_edit_status() -> Result<bool, String> {
    native_task(move || overlay_edit_status_blocking()).await
}

fn overlay_edit_status_blocking() -> Result<bool, String> {
    let response = send_host_command("edit_status")
        .map_err(|error| format!("Could not reach overlay host: {error}"))?;
    HostReply::parse(&response)
        .and_then(|reply| reply.edit_mode())
        .ok_or_else(|| format!("host returned '{response}'"))
}

#[tauri::command]
async fn toggle_overlay_visibility() -> Result<bool, String> {
    native_task(move || toggle_overlay_visibility_blocking()).await
}

fn toggle_overlay_visibility_blocking() -> Result<bool, String> {
    let response = send_host_command("toggle_visibility")
        .map_err(|error| format!("Could not reach overlay host: {error}"))?;
    HostReply::parse(&response)
        .and_then(|reply| reply.visibility())
        .ok_or_else(|| format!("host returned '{response}'"))
}

#[tauri::command]
async fn overlay_visibility_status() -> Result<bool, String> {
    native_task(move || overlay_visibility_status_blocking()).await
}

fn overlay_visibility_status_blocking() -> Result<bool, String> {
    let response = send_host_command("visibility_status")
        .map_err(|error| format!("Could not reach overlay host: {error}"))?;
    HostReply::parse(&response)
        .and_then(|reply| reply.visibility())
        .ok_or_else(|| format!("host returned '{response}'"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum HostState {
    Running,
    Stopped,
    Unreachable,
}
#[derive(Debug, Serialize)]
struct HostStateResponse {
    state: HostState,
    visible: bool,
    editing: bool,
    error: Option<String>,
}

#[tauri::command]
async fn host_state() -> Result<HostStateResponse, String> {
    native_task(query_host_state).await
}

fn query_host_state() -> Result<HostStateResponse, String> {
    Ok(host_state_from_reply(send_host_command("snapshot"), host_is_running()))
}

fn host_state_from_reply(response: std::io::Result<String>, process_present: bool) -> HostStateResponse {
    match response {
        Ok(reply) => match HostReply::parse(&reply) {
            Some(HostReply::Snapshot(snapshot)) => HostStateResponse {
                state: if snapshot.running { HostState::Running } else { HostState::Stopped },
                visible: snapshot.running && snapshot.visible, editing: snapshot.running && snapshot.editing, error: None,
            },
            _ => HostStateResponse { state: HostState::Unreachable, visible: false, editing: false, error: Some("Overlay host returned an unsupported state snapshot. Restart the updated overlay.".into()) },
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !process_present => HostStateResponse { state: HostState::Stopped, visible: false, editing: false, error: None },
        Err(error) => HostStateResponse { state: HostState::Unreachable, visible: false, editing: false, error: Some(format!("Could not read overlay state: {error}")) },
    }
}

fn reload_overlay_config_after_save() -> Result<(), String> {
    if !host_control_is_ready() {
        if host_is_running() {
            return Err("Configuration was saved, but the overlay host is not responding. Restart the host and press Save to retry.".to_string());
        }
        return Ok(());
    }
    let response = send_host_command("reload")
        .map_err(|error| format!("Configuration was saved, but overlay reload failed: {error}"))?;
    validate_reload_response(&response)
        .map_err(|error| format!("Configuration was saved, but overlay reload failed: {error}"))
}

fn validate_reload_response(response: &str) -> Result<(), String> {
    if HostReply::parse(response) == Some(HostReply::Reloaded) {
        Ok(())
    } else {
        Err(format!("host returned '{response}'"))
    }
}

fn host_ready_response(response: &str) -> bool {
    HostReply::parse(response) == Some(HostReply::Running)
}

fn host_control_is_ready() -> bool {
    send_host_command("status").is_ok_and(|response| host_ready_response(&response))
}

fn startup_decision(host_control_ready: bool) -> StartupDecision {
    if host_control_ready {
        StartupDecision::ReuseReadyHost
    } else {
        StartupDecision::Launch
    }
}

#[cfg(test)]
mod startup_status_tests {
    use super::{host_ready_response, startup_decision, validate_reload_response, StartupDecision};

    #[test]
    fn only_reports_running_after_host_control_confirms_ready() {
        assert!(host_ready_response("running"));
        assert!(!host_ready_response("stopped"));
        assert!(!host_ready_response("stopping"));
        assert!(!host_ready_response("invalid"));
        assert!(!host_ready_response("error:window creation failed"));
    }

    #[test]
    fn launches_when_a_mutex_is_not_backed_by_a_ready_control_service() {
        assert_eq!(startup_decision(false), StartupDecision::Launch);
        assert_eq!(startup_decision(true), StartupDecision::ReuseReadyHost);
    }

    #[test]
    fn rejects_unsuccessful_runtime_reload_responses() {
        assert!(validate_reload_response("reloaded").is_ok());
        assert!(validate_reload_response("running").is_err());
        assert!(validate_reload_response("error:host stopped").is_err());
        assert!(validate_reload_response("error:reload timed out").is_err());
    }
}

fn send_host_command(command: &str) -> std::io::Result<String> {
    let command = HostCommand::parse(command).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "unsupported host command")
    })?;
    host_client::send(command)
}

#[tauri::command]
async fn list_displays() -> Result<Vec<displays::Display>, String> {
    native_task(displays::list).await
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

fn wait_for_host_startup() -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if host_control_is_ready() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("HashOverlay could not confirm that its overlay window is ready. If a previous HashOverlay process is still open, close it and try Start overlay again.".to_string())
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

fn finish_saved_config(
    path: PathBuf,
    reload_result: Result<(), String>,
) -> Result<ConfigResponse, String> {
    let mut response = ConfigResponse::read(path)?;
    response.reload_warning = reload_result.err();
    Ok(response)
}

#[cfg(test)]
mod save_feedback_tests {
    use super::*;

    #[test]
    fn reset_preserves_external_edits_and_accepts_the_latest_revision() {
        let path = std::env::temp_dir().join(format!(
            "hashoverlay-reset-conflict-{}.toml",
            std::process::id()
        ));
        let mut config = OverlayConfig::default();
        config.save(&path).unwrap();
        let stale = OverlayConfig::revision(&path).unwrap();
        config.window.x = 123;
        config.save(&path).unwrap();
        let external = fs::read(&path).unwrap();
        assert!(reset_config_at(&path, stale).is_err());
        assert_eq!(fs::read(&path).unwrap(), external);
        let latest = OverlayConfig::revision(&path).unwrap();
        reset_config_at(&path, latest).unwrap();
        assert_eq!(
            OverlayConfig::load(&path).unwrap().window.x,
            OverlayConfig::default().window.x
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn recovery_validation_normalizes_geometry_without_accepting_future_schemas() {
        let mut candidate = OverlayConfig::default();
        candidate.window.width = 0;
        let recovered = validate_config(candidate.clone()).unwrap();
        assert!(recovered.window.width > 0);
        assert_eq!(candidate.window.width, 0);
        candidate.config_version = u32::MAX;
        assert!(validate_config(candidate).unwrap_err().contains("version"));
    }

    #[test]
    fn recovery_validation_rejects_conflicting_hotkeys() {
        let mut candidate = OverlayConfig::default();
        candidate.hotkeys.edit_mode = candidate.hotkeys.toggle_overlay.clone();
        assert!(validate_config(candidate).is_err());
    }

    #[test]
    fn import_preview_migrates_and_normalizes_without_accepting_invalid_candidates() {
        let preview = preview_import("config_version = 3\n[window]\nwidth = 0\n".into()).unwrap();
        assert!(preview.config_version > 3);
        assert!(preview.window.width >= 280);
        assert!(!preview.overlays.is_empty());
        for text in [
            "",
            "not TOML",
            "config_version = 999999",
            "[hotkeys]\ntoggle_overlay = 'F9'\nedit_mode = 'F9'",
        ] {
            assert!(preview_import(text.into()).is_err(), "{text}");
        }
        assert!(preview_import("#".repeat(4 * 1024 * 1024 + 1)).is_err());
        assert!(export_config(OverlayConfig {
            config_version: u32::MAX,
            ..OverlayConfig::default()
        })
        .is_err());
    }

    #[test]
    fn native_host_state_contract_distinguishes_absence_from_a_stalled_process() {
        let absent = || std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let stopped = serde_json::to_value(host_state_from_reply(Err(absent()), false)).unwrap();
        assert_eq!(stopped["state"], "stopped");
        assert_eq!(stopped["visible"], false);
        let stalled = serde_json::to_value(host_state_from_reply(Err(absent()), true)).unwrap();
        assert_eq!(stalled["state"], "unreachable");
        let live = serde_json::to_value(host_state_from_reply(Ok("state:1:0:1".into()), true)).unwrap();
        assert_eq!(live["state"], "running");
        assert_eq!(live["editing"], true);
        assert_eq!(live["visible"], false);
        assert!(live["error"].is_null());
        let invalid = serde_json::to_value(host_state_from_reply(Ok("invalid".into()), true)).unwrap();
        assert_eq!(invalid["state"], "unreachable");
    }

    #[test]
    fn snapshot_read_errors_never_become_a_valid_zero_revision() {
        let missing = std::env::temp_dir().join(format!(
            "hashoverlay-missing-snapshot-{}.toml",
            std::process::id()
        ));
        assert!(ConfigResponse::read(missing).is_err());
    }

    #[test]
    fn revision_tokens_reject_rounded_numbers_and_preserve_the_full_u64_range() {
        assert_eq!(parse_revision("18446744073709551615").unwrap(), u64::MAX);
        assert_eq!(parse_revision("0").unwrap(), 0);
        for invalid in ["", "-1", "1.5", "1e9", " 1", "18446744073709551616"] {
            assert!(parse_revision(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn revisions_cross_json_as_lossless_decimal_tokens() {
        let response = ConfigResponse {
            path: "test.toml".into(),
            config: OverlayConfig::default(),
            revision: u64::MAX.to_string(),
            reload_warning: None,
        };
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["revision"].as_str(), Some("18446744073709551615"));
    }

    #[test]
    fn a_reload_failure_still_returns_the_saved_config_and_revision() {
        let path = std::env::temp_dir().join(format!(
            "hashoverlay-reload-warning-{}.toml",
            std::process::id()
        ));
        let mut config = OverlayConfig::default();
        config.window.x = 42;
        config.save(&path).unwrap();
        let response = finish_saved_config(path.clone(), Err("host reload failed".into())).unwrap();
        assert_eq!(response.config.window.x, 42);
        assert_eq!(
            response.revision,
            OverlayConfig::revision(&path).unwrap().to_string()
        );
        assert_eq!(
            response.reload_warning.as_deref(),
            Some("host reload failed")
        );
        assert!(finish_saved_config(path.clone(), Ok(()))
            .unwrap()
            .reload_warning
            .is_none());
        fs::remove_file(path).unwrap();
    }
}

impl ConfigResponse {
    fn read(path: PathBuf) -> Result<Self, String> {
        for _ in 0..3 {
            let before = OverlayConfig::revision(&path).map_err(|error| error.to_string())?;
            let config = OverlayConfig::load(&path).map_err(|error| error.to_string())?;
            let after = OverlayConfig::revision(&path).map_err(|error| error.to_string())?;
            if before == after {
                return Ok(Self {
                    path: path.display().to_string(),
                    config,
                    revision: after.to_string(),
                    reload_warning: None,
                });
            }
        }
        Err("Configuration kept changing while being read. Wait for the other editor to finish, then reload.".into())
    }
}

fn parse_revision(value: &str) -> Result<u64, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Invalid configuration revision. Reload the configuration and retry.".into());
    }
    value.parse().map_err(|_| {
        "Configuration revision is out of range. Reload the configuration and retry.".into()
    })
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
            validate_config,
            export_config,
            import_config,
            preview_import,
            reset_config,
            widget_catalog,
            list_displays,
            start_overlay,
            stop_overlay,
            overlay_status,
            host_state,
            toggle_overlay_edit_mode,
            overlay_edit_status,
            toggle_overlay_visibility,
            overlay_visibility_status,
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

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};

    #[test]
    fn frontend_revision_arguments_reach_save_and_import_validation() {
        let app = mock_builder()
            .invoke_handler(tauri::generate_handler![
                save_config,
                import_config,
                validate_config
            ])
            .build(mock_context(noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let future = OverlayConfig {
            config_version: u32::MAX,
            ..OverlayConfig::default()
        };
        for (cmd, body) in [
            (
                "save_config",
                serde_json::json!({ "config": future, "expected_revision": "0" }),
            ),
            (
                "import_config",
                serde_json::json!({ "text": "not valid TOML", "expected_revision": "0" }),
            ),
        ] {
            let error = get_ipc_response(
                &webview,
                tauri::webview::InvokeRequest {
                    cmd: cmd.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "http://tauri.localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.into(),
                },
            )
            .unwrap_err()
            .to_string();
            assert!(!error.contains("invalid args"), "{cmd}: {error}");
            assert!(!error.contains("missing required key"), "{cmd}: {error}");
        }
    }
}

async fn native_task<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("Native operation failed: {error}"))?
}
