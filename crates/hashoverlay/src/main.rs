use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process::{Command, ExitCode},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use anyhow::Result;
use lap_engine::{LapEngine, LapEngineConfig, ReferenceMode};
use lmu_telemetry::{
    format_sample_line, SharedMemoryTelemetrySource, TelemetrySample, TelemetrySource,
};
use log::{info, warn};
use overlay_renderer::{
    config::OverlayConfig, HostRuntimeStats, SharedRuntimeView, TelemetryOverlay,
};
use storage::{ReferenceLapKey, ReferenceLapStore};
use telemetry_engine::{FuelEngine, RingBuffer, TelemetrySnapshot};

mod host_control;
mod host_hotkeys;
mod host_instance;

#[derive(Clone)]
struct HostRuntime {
    latest: Arc<Mutex<Option<TelemetrySnapshot>>>,
    history: Arc<Mutex<RingBuffer<TelemetrySnapshot>>>,
    stats: Arc<Mutex<HostRuntimeStats>>,
    running: Arc<AtomicBool>,
    visible: Arc<AtomicBool>,
    edit_mode: Arc<AtomicBool>,
}

struct SurfaceHandle {
    running: Arc<AtomicBool>,
    join: thread::JoinHandle<Result<(), overlay_renderer::OverlayError>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    overlay: bool,
    overlay_layer: Option<String>,
    configure: bool,
    print_config_path: bool,
    version: bool,
    config_path: Option<PathBuf>,
    once: bool,
    wait: bool,
    interval: Duration,
}

fn main() -> ExitCode {
    env_logger::init();

    match run(Cli::parse(std::env::args().skip(1))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    if cli.configure {
        return open_overlay_config(cli.config_path);
    }

    if cli.print_config_path {
        println!("{}", overlay_config_path(cli.config_path).display());
        return Ok(());
    }

    if cli.version {
        println!("HashOverlay {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if cli.overlay {
        let Some(_host_instance) = host_instance::HostInstance::acquire()? else {
            info!("HashOverlay host is already running; not starting a duplicate");
            return Ok(());
        };
        return run_overlay(cli.config_path, cli.overlay_layer);
    }

    let mut source = SharedMemoryTelemetrySource::open()?;

    if !source.is_available() {
        warn!("LMU telemetry buffer is not available. Start LMU and enter a driving session.");
        if !cli.wait {
            return Ok(());
        }
    }

    let mut detected = source.is_available();
    if detected {
        info!("LMU telemetry buffer detected.");
    }

    loop {
        match source.read_sample() {
            Ok(Some(sample)) => {
                if !detected {
                    detected = true;
                    info!("LMU telemetry buffer detected.");
                }
                println!("{}", format_sample_line(&sample));
            }
            Ok(None) if detected => {
                warn!("Telemetry buffer is present, but no player sample is available yet.");
            }
            Ok(None) => {}
            Err(error) => warn!("Could not read telemetry sample: {error}"),
        }

        if cli.once && detected {
            break;
        }

        thread::sleep(cli.interval);
    }

    Ok(())
}

impl Cli {
    fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut cli = Self {
            overlay: false,
            overlay_layer: None,
            configure: false,
            print_config_path: false,
            version: false,
            config_path: None,
            once: false,
            wait: false,
            interval: Duration::from_millis(100),
        };

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--overlay" => cli.overlay = true,
                "--overlay-layer" => cli.overlay_layer = args.next(),
                "--configure" => cli.configure = true,
                "--print-config-path" => cli.print_config_path = true,
                "--version" | "-V" => cli.version = true,
                "--config" => {
                    if let Some(value) = args.next() {
                        cli.config_path = Some(PathBuf::from(value));
                    }
                }
                "--once" => cli.once = true,
                "--wait" => cli.wait = true,
                "--interval-ms" => {
                    if let Some(value) = args.next().and_then(|value| value.parse::<u64>().ok()) {
                        cli.interval = Duration::from_millis(value.max(1));
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {}
            }
        }

        cli
    }
}

fn print_help() {
    println!(
        "\
LMU Overlay telemetry probe

Usage:
  hashoverlay [--overlay] [--overlay-layer <id>] [--configure] [--config <path>] [--once] [--wait] [--interval-ms <milliseconds>]

Options:
  --overlay                 Open the transparent always-on-top telemetry overlay.
  --overlay-layer <id>     Open only one configured overlay surface.
  --configure               Create and open the user overlay config.
  --print-config-path       Print the default overlay config path.
  -V, --version             Print the HashOverlay version.
  --config <path>           Use a custom overlay config path.
  --once                    Print one telemetry sample and exit after telemetry is detected.
  --wait                    Keep waiting when LMU telemetry is not available yet.
  --interval-ms <value>     Poll interval for CLI logging. Default: 100.
  -h, --help                Show this help."
    );
}

fn run_overlay(config_path: Option<PathBuf>, overlay_layer: Option<String>) -> Result<()> {
    let mut source = SharedMemoryTelemetrySource::open()?;
    let config_path = overlay_config_path(config_path);
    OverlayConfig::save_default(&config_path)?;
    let config = OverlayConfig::load(&config_path)?;
    let (host_action_sender, host_action_receiver) = mpsc::channel();
    let mut active_hotkeys = config.hotkeys.clone();
    let mut host_hotkeys = Some(host_hotkeys::HostHotkeys::start(
        active_hotkeys.clone(),
        host_action_sender.clone(),
    )?);
    let surface_configs = match enabled_surface_configs(&config, overlay_layer.as_deref()) {
        Ok(configs) => configs,
        Err(error) => {
            if let Some(hotkeys) = host_hotkeys.take() {
                hotkeys.shutdown();
            }
            return Err(error);
        }
    };
    let shared_latest = Arc::new(Mutex::new(None));
    let history_capacity = surface_configs
        .iter()
        .map(|(_, config)| config.window.history_samples)
        .max()
        .unwrap_or(16);
    let shared_history = Arc::new(Mutex::new(RingBuffer::new(history_capacity)));
    let shared_stats = Arc::new(Mutex::new(HostRuntimeStats::default()));
    let shared_visible = Arc::new(AtomicBool::new(true));
    let shared_edit_mode = Arc::new(AtomicBool::new(false));
    let reload_requested = Arc::new(AtomicBool::new(false));
    let (reload_request_sender, reload_request_receiver) = mpsc::channel();
    let host_running = Arc::new(AtomicBool::new(true));
    let runtime = HostRuntime {
        latest: shared_latest.clone(),
        history: shared_history.clone(),
        stats: shared_stats.clone(),
        running: host_running.clone(),
        visible: shared_visible.clone(),
        edit_mode: shared_edit_mode.clone(),
    };
    let lap_store = ReferenceLapStore::appdata();
    let lap_writer_store = lap_store.clone();
    let (lap_writer, lap_receiver) = mpsc::channel();
    let runtime_lap_writer = lap_writer.clone();
    let lap_writer_handle = thread::spawn(move || {
        while let Ok((lap_key, lap)) = lap_receiver.recv() {
            if let Err(error) = lap_writer_store.save_personal_best(&lap_key, &lap) {
                warn!("Could not save personal best reference lap: {error}");
            }
        }
    });

    let acquisition_path = config_path.clone();
    let acquisition_layer = overlay_layer.clone();
    let acquisition_latest = shared_latest.clone();
    let acquisition_history = shared_history.clone();
    let acquisition_stats = shared_stats.clone();
    let acquisition_running = host_running.clone();
    let acquisition_handle = thread::spawn(move || {
        let mut current_config = config;
        let mut lap_config = lap_engine_config(&current_config);
        let mut current_lap_key = None;
        let mut lap_engine = LapEngine::new(lap_config.clone());
        let mut fuel_engine = FuelEngine::default();
        let mut last_config_check = Instant::now();
        let mut config_mtime = modified_time(&acquisition_path);
        let mut next_sample = Instant::now();
        let mut last_stats_refresh = Instant::now();

        while acquisition_running.load(Ordering::Relaxed) {
            if last_config_check.elapsed() >= Duration::from_millis(500) {
                if let Some((next_config, mtime)) = load_config_if_changed(
                    &acquisition_path,
                    config_mtime,
                    acquisition_layer.as_deref(),
                ) {
                    let next_lap_config = lap_engine_config(&next_config);
                    if next_lap_config != lap_config {
                        lap_config = next_lap_config;
                        lap_engine.update_config(lap_config.clone());
                    }
                    current_config = next_config;
                    config_mtime = Some(mtime);
                }
                last_config_check = Instant::now();
            }

            if Instant::now() < next_sample {
                thread::sleep(next_sample - Instant::now());
                continue;
            }
            let sample_interval = Duration::from_millis(current_config.window.sample_ms.max(5));
            next_sample += sample_interval;

            let acquisition_started = Instant::now();
            match source.read_sample() {
                Ok(Some(sample)) => {
                    let lap_key = reference_lap_key(&sample);
                    if current_lap_key.as_ref() != Some(&lap_key) {
                        let personal_best = match lap_store.load_personal_best(&lap_key) {
                            Ok(personal_best) => personal_best,
                            Err(error) => {
                                warn!("Could not load personal best reference lap: {error}");
                                None
                            }
                        };
                        lap_engine =
                            LapEngine::new(lap_config.clone()).with_personal_best(personal_best);
                        current_lap_key = Some(lap_key.clone());
                    }

                    let fuel = fuel_engine.update(&sample);
                    let mut snapshot = TelemetrySnapshot::from(sample);
                    snapshot.fuel_last_lap_used = fuel.last_lap_used;
                    snapshot.fuel_average_lap_used = fuel.average_lap_used;
                    snapshot.fuel_estimated_laps_remaining = fuel.estimated_laps_remaining;
                    lap_engine.update(snapshot.clone()).apply_to(&mut snapshot);
                    if let Some(lap) = lap_engine.take_new_personal_best() {
                        if let Some(lap_key) = &current_lap_key {
                            let _ = runtime_lap_writer.send((lap_key.clone(), lap));
                        }
                    }
                    if let Ok(mut latest) = acquisition_latest.lock() {
                        *latest = Some(snapshot.clone());
                    }
                    if let Ok(mut history) = acquisition_history.lock() {
                        history.push(snapshot);
                    }
                    if let Ok(mut stats) = acquisition_stats.lock() {
                        stats.record_sample(acquisition_started.elapsed());
                    }
                }
                Ok(None) => {}
                Err(error) => warn!("Could not read telemetry sample: {error}"),
            }
            let now = Instant::now();
            if next_sample <= now {
                let skipped = (now.duration_since(next_sample).as_nanos()
                    / sample_interval.as_nanos())
                .saturating_add(1)
                .min(u64::MAX as u128) as u64;
                if let Ok(mut stats) = acquisition_stats.lock() {
                    stats.record_skipped_samples(skipped);
                }
                next_sample = now + sample_interval;
            }
            if last_stats_refresh.elapsed() >= Duration::from_secs(1) {
                if let Ok(mut stats) = acquisition_stats.lock() {
                    stats.refresh();
                }
                last_stats_refresh = Instant::now();
            }
        }
    });

    let mut surfaces = HashMap::new();
    if let Err(error) = reconcile_surfaces(
        &config_path,
        overlay_layer.as_deref(),
        &runtime,
        &mut surfaces,
    ) {
        host_running.store(false, Ordering::Relaxed);
        let _ = acquisition_handle.join();
        drop(lap_writer);
        let _ = lap_writer_handle.join();
        if let Some(hotkeys) = host_hotkeys.take() {
            hotkeys.shutdown();
        }
        return Err(error);
    }
    // Publish host readiness only after the initial overlay surface has
    // created its window. Settings uses this pipe as the startup handshake.
    let host_control = match host_control::HostControl::start(
        host_running.clone(),
        shared_visible.clone(),
        shared_edit_mode.clone(),
        reload_request_sender,
    ) {
        Ok(control) => control,
        Err(error) => {
            // Surface startup already succeeded, so a control-pipe failure must
            // explicitly stop every worker before returning to the launcher.
            // Otherwise their JoinHandles are detached while the host process
            // reports a failed startup.
            host_running.store(false, Ordering::Relaxed);
            for (_, surface) in surfaces.drain() {
                surface.running.store(false, Ordering::Relaxed);
                let _ = surface.join.join();
            }
            let _ = acquisition_handle.join();
            drop(lap_writer);
            let _ = lap_writer_handle.join();
            if let Some(hotkeys) = host_hotkeys.take() {
                hotkeys.shutdown();
            }
            return Err(error.into());
        }
    };
    let mut last_reconcile = Instant::now();
    while host_running.load(Ordering::Relaxed) {
        while let Ok(reply) = reload_request_receiver.try_recv() {
            let result = reload_host_hotkeys_if_changed(
                &config_path,
                &mut active_hotkeys,
                &mut host_hotkeys,
                &host_action_sender,
            )
            .and_then(|()| {
                reconcile_surfaces(
                    &config_path,
                    overlay_layer.as_deref(),
                    &runtime,
                    &mut surfaces,
                )
            });
            let _ = reply.send(result.map_err(|error| error.to_string()));
            last_reconcile = Instant::now();
        }
        while let Ok(action) = host_action_receiver.try_recv() {
            match action {
                host_hotkeys::HostAction::ToggleVisibility => {
                    shared_visible.fetch_xor(true, Ordering::Relaxed);
                }
                host_hotkeys::HostAction::ToggleEditMode => {
                    let enabled = !shared_edit_mode.load(Ordering::Relaxed);
                    shared_edit_mode.store(enabled, Ordering::Relaxed);
                    if enabled {
                        shared_visible.store(true, Ordering::Relaxed);
                    }
                }
                host_hotkeys::HostAction::ToggleCoaching => {
                    if let Err(error) = toggle_host_coaching(&config_path) {
                        warn!("Could not toggle coaching from host hotkey: {error}");
                    } else {
                        reload_requested.store(true, Ordering::Relaxed);
                    }
                }
                host_hotkeys::HostAction::CyclePreset => {
                    if let Err(error) = cycle_host_preset(&config_path) {
                        warn!("Could not cycle preset from host hotkey: {error}");
                    } else {
                        reload_requested.store(true, Ordering::Relaxed);
                    }
                }
            }
        }
        if reload_requested.swap(false, Ordering::Relaxed)
            || last_reconcile.elapsed() >= Duration::from_millis(500)
        {
            if let Err(error) = reload_host_hotkeys_if_changed(
                &config_path,
                &mut active_hotkeys,
                &mut host_hotkeys,
                &host_action_sender,
            ) {
                warn!("Could not reload host hotkeys: {error}");
            }
            if let Err(error) = reconcile_surfaces(
                &config_path,
                overlay_layer.as_deref(),
                &runtime,
                &mut surfaces,
            ) {
                warn!("Could not reconcile overlay surfaces: {error}");
            }
            last_reconcile = Instant::now();
        }
        reap_finished_surfaces(&mut surfaces);
        thread::sleep(Duration::from_millis(25));
    }
    for (_, surface) in surfaces {
        surface.running.store(false, Ordering::Relaxed);
        if let Err(error) = surface
            .join
            .join()
            .unwrap_or(Err(overlay_renderer::OverlayError::UnsupportedPlatform))
        {
            warn!("Overlay surface stopped: {error}");
        }
    }
    host_running.store(false, Ordering::Relaxed);
    let _ = acquisition_handle.join();
    drop(lap_writer);
    if let Err(error) = lap_writer_handle.join() {
        warn!("Could not join personal best storage worker: {error:?}");
    }
    host_control.shutdown();
    if let Some(hotkeys) = host_hotkeys {
        hotkeys.shutdown();
    }

    Ok(())
}

fn toggle_host_coaching(config_path: &PathBuf) -> Result<()> {
    let mut config = OverlayConfig::load(config_path)?;
    config.widgets.coaching = !config.widgets.coaching;
    config.coaching.mode = if config.widgets.coaching {
        "practice".to_string()
    } else {
        "off".to_string()
    };
    let revision = OverlayConfig::revision(config_path)?;
    config.save_if_revision(config_path, revision)?;
    Ok(())
}

fn cycle_host_preset(config_path: &PathBuf) -> Result<()> {
    let mut config = OverlayConfig::load(config_path)?;
    config.cycle_preset();
    let revision = OverlayConfig::revision(config_path)?;
    config.save_if_revision(config_path, revision)?;
    Ok(())
}

fn reload_host_hotkeys_if_changed(
    config_path: &PathBuf,
    active_hotkeys: &mut overlay_renderer::config::HotkeyConfig,
    host_hotkeys: &mut Option<host_hotkeys::HostHotkeys>,
    actions: &mpsc::Sender<host_hotkeys::HostAction>,
) -> Result<()> {
    let config = OverlayConfig::load(config_path)?;
    if config.hotkeys == *active_hotkeys {
        return Ok(());
    }
    if let Some(hotkeys) = host_hotkeys.take() {
        hotkeys.shutdown();
    }
    *active_hotkeys = config.hotkeys.clone();
    *host_hotkeys = Some(host_hotkeys::HostHotkeys::start(
        active_hotkeys.clone(),
        actions.clone(),
    )?);
    Ok(())
}

fn load_config_if_changed(
    config_path: &PathBuf,
    previous_mtime: Option<SystemTime>,
    overlay_layer: Option<&str>,
) -> Option<(OverlayConfig, SystemTime)> {
    let mtime = modified_time(config_path)?;
    if previous_mtime.is_some_and(|previous| previous >= mtime) {
        return None;
    }
    match OverlayConfig::load(config_path) {
        Ok(config) => match config.for_overlay_layer(overlay_layer) {
            Ok(config) => Some((config, mtime)),
            Err(error) => {
                warn!("Could not select overlay layer: {error}");
                None
            }
        },
        Err(error) => {
            warn!("Could not hot reload overlay timing config: {error}");
            None
        }
    }
}

fn reconcile_surfaces(
    config_path: &PathBuf,
    selected_layer: Option<&str>,
    runtime: &HostRuntime,
    surfaces: &mut HashMap<String, SurfaceHandle>,
) -> Result<()> {
    let config = OverlayConfig::load(config_path)?;
    let desired = enabled_surface_configs(&config, selected_layer)?;
    let desired_ids = desired
        .iter()
        .filter_map(|(id, _)| id.clone())
        .collect::<std::collections::HashSet<_>>();
    let history_capacity = desired
        .iter()
        .map(|(_, config)| config.window.history_samples)
        .max()
        .unwrap_or(16);
    if let Ok(mut history) = runtime.history.lock() {
        history.resize_preserving_recent(history_capacity);
    }

    let removed = surfaces
        .keys()
        .filter(|id| !desired_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    for id in removed {
        if let Some(surface) = surfaces.remove(&id) {
            surface.running.store(false, Ordering::Relaxed);
            let _ = surface.join.join();
        }
    }
    for (layer_id, surface_config) in desired {
        let id = layer_id.unwrap_or_else(|| "main".to_string());
        if surfaces.contains_key(&id) {
            continue;
        }
        let surface = TelemetryOverlay::with_config_path_and_layer(
            surface_config,
            config_path.clone(),
            Some(id.clone()),
        )?;
        let surface_running = Arc::new(AtomicBool::new(true));
        let view = SharedRuntimeView {
            latest: runtime.latest.clone(),
            history: runtime.history.clone(),
            host_stats: runtime.stats.clone(),
            host_running: runtime.running.clone(),
            surface_running: surface_running.clone(),
            visible: runtime.visible.clone(),
            edit_mode: runtime.edit_mode.clone(),
        };
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let join = thread::spawn(move || surface.run_shared(view, Some(ready_sender)));
        match ready_receiver.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                surface_running.store(false, Ordering::Relaxed);
                let _ = join.join();
                for (_, surface) in surfaces.drain() {
                    surface.running.store(false, Ordering::Relaxed);
                    let _ = surface.join.join();
                }
                return Err(anyhow::anyhow!(
                    "overlay surface '{id}' failed to start: {error}"
                ));
            }
            Err(error) => {
                surface_running.store(false, Ordering::Relaxed);
                if !join.is_finished() {
                    runtime.running.store(false, Ordering::Relaxed);
                }
                let _ = join.join();
                for (_, surface) in surfaces.drain() {
                    surface.running.store(false, Ordering::Relaxed);
                    let _ = surface.join.join();
                }
                return Err(anyhow::anyhow!(
                    "overlay surface '{id}' did not report startup: {error}"
                ));
            }
        }
        surfaces.insert(
            id,
            SurfaceHandle {
                running: surface_running,
                join,
            },
        );
    }
    Ok(())
}

fn reap_finished_surfaces(surfaces: &mut HashMap<String, SurfaceHandle>) {
    let finished = surfaces
        .iter()
        .filter_map(|(id, surface)| surface.join.is_finished().then_some(id.clone()))
        .collect::<Vec<_>>();
    for id in finished {
        if let Some(surface) = surfaces.remove(&id) {
            match surface.join.join() {
                Ok(Ok(())) => warn!("Overlay surface '{id}' exited"),
                Ok(Err(error)) => warn!("Overlay surface '{id}' failed: {error}"),
                Err(error) => warn!("Overlay surface '{id}' panicked: {error:?}"),
            }
        }
    }
}

fn modified_time(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn enabled_surface_configs(
    config: &OverlayConfig,
    selected_layer: Option<&str>,
) -> Result<Vec<(Option<String>, OverlayConfig)>> {
    if let Some(layer_id) = selected_layer {
        return Ok(vec![(
            Some(layer_id.to_string()),
            config.for_overlay_layer(Some(layer_id))?,
        )]);
    }

    config
        .overlays
        .iter()
        .filter(|layer| layer.enabled)
        .map(|layer| {
            Ok((
                Some(layer.id.clone()),
                config.for_overlay_layer(Some(layer.id.as_str()))?,
            ))
        })
        .collect()
}

fn reference_lap_key(sample: &TelemetrySample) -> ReferenceLapKey {
    ReferenceLapKey {
        track: sample
            .metadata
            .track_name
            .clone()
            .unwrap_or_else(|| "unknown-track".to_string()),
        track_layout: sample.metadata.track_layout.clone().unwrap_or_default(),
        car: sample
            .metadata
            .vehicle_name
            .clone()
            .unwrap_or_else(|| "unknown-car".to_string()),
        legacy_vehicle_class: sample.metadata.vehicle_class.clone(),
    }
}

fn lap_engine_config(config: &OverlayConfig) -> LapEngineConfig {
    LapEngineConfig {
        reference_mode: match config.timing.reference_mode.as_str() {
            "session_best" => ReferenceMode::SessionBest,
            "best_valid_lap" => ReferenceMode::BestValidLap,
            "last_lap" => ReferenceMode::LastLap,
            _ => ReferenceMode::PersonalBest,
        },
        mini_sectors: config.timing.mini_sectors,
        brake_threshold: config.timing.brake_threshold,
        throttle_threshold: config.timing.throttle_threshold,
        event_match_tolerance_m: config.coaching.event_match_tolerance_m,
        ..LapEngineConfig::default()
    }
}

fn open_overlay_config(config_path: Option<PathBuf>) -> Result<()> {
    let config_path = overlay_config_path(config_path);
    OverlayConfig::save_default(&config_path)?;
    let settings_candidates = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .into_iter()
        .flat_map(|parent| {
            [
                parent.join("hashoverlay-settings.exe"),
                parent.join("HashOverlay Settings.exe"),
            ]
        })
        .collect::<Vec<_>>();

    if let Some(settings) = settings_candidates.into_iter().find(|path| path.exists()) {
        Command::new(settings).spawn()?;
        return Ok(());
    }

    println!(
        "Settings app was not found. Opening {}",
        config_path.display()
    );
    Command::new("notepad").arg(&config_path).spawn()?;
    Ok(())
}

fn overlay_config_path(config_path: Option<PathBuf>) -> PathBuf {
    if let Some(path) = config_path {
        return path;
    }

    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("HashOverlay")
        .join("hashoverlay.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use overlay_renderer::config::OverlayLayerConfig;

    #[test]
    fn parses_default_cli() {
        let cli = Cli::parse(Vec::new());

        assert!(!cli.once);
        assert!(!cli.overlay);
        assert!(!cli.configure);
        assert!(!cli.print_config_path);
        assert!(cli.config_path.is_none());
        assert!(!cli.wait);
        assert_eq!(cli.interval, Duration::from_millis(100));
    }

    #[test]
    fn parses_cli_flags() {
        let cli = Cli::parse([
            "--once".to_string(),
            "--wait".to_string(),
            "--overlay".to_string(),
            "--configure".to_string(),
            "--print-config-path".to_string(),
            "--config".to_string(),
            "custom.toml".to_string(),
            "--interval-ms".to_string(),
            "25".to_string(),
        ]);

        assert!(cli.once);
        assert!(cli.wait);
        assert!(cli.overlay);
        assert!(cli.configure);
        assert!(cli.print_config_path);
        assert_eq!(cli.config_path, Some(PathBuf::from("custom.toml")));
        assert_eq!(cli.interval, Duration::from_millis(25));
    }

    #[test]
    fn builds_reference_lap_key_from_telemetry_metadata() {
        let sample = TelemetrySample {
            timestamp_seconds: 0.0,
            speed_mps: 0.0,
            rpm: 0.0,
            gear: lmu_telemetry::Gear::Neutral,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            lap_distance_m: None,
            track_length_m: None,
            lap_number: 1,
            lap_start_seconds: 0.0,
            sector: 0,
            sector_times: Default::default(),
            vehicle: Default::default(),
            wheels: Default::default(),
            session: Default::default(),
            metadata: lmu_telemetry::TelemetryMetadata {
                track_name: Some("Sebring".to_string()),
                track_layout: Some("International".to_string()),
                vehicle_name: Some("Porsche 963".to_string()),
                vehicle_class: Some("Hypercar".to_string()),
                ..lmu_telemetry::TelemetryMetadata::default()
            },
            field: std::sync::Arc::from(Vec::new()),
        };

        assert_eq!(
            reference_lap_key(&sample),
            ReferenceLapKey {
                track: "Sebring".to_string(),
                track_layout: "International".to_string(),
                car: "Porsche 963".to_string(),
                legacy_vehicle_class: Some("Hypercar".to_string()),
            }
        );
    }

    #[test]
    fn host_selects_only_enabled_surfaces_and_rejects_unknown_debug_layers() {
        let mut config = OverlayConfig {
            overlays: vec![
                OverlayLayerConfig {
                    id: "main".to_string(),
                    enabled: true,
                    ..OverlayLayerConfig::default()
                },
                OverlayLayerConfig {
                    id: "race".to_string(),
                    enabled: false,
                    ..OverlayLayerConfig::default()
                },
            ],
            ..OverlayConfig::default()
        };
        config.normalize();

        let surfaces = enabled_surface_configs(&config, None).unwrap();
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].0.as_deref(), Some("main"));
        assert!(enabled_surface_configs(&config, Some("missing")).is_err());
    }
}
