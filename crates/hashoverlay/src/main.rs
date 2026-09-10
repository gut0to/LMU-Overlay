use std::{
    path::PathBuf,
    process::{Command, ExitCode},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Result;
use lap_engine::{LapEngine, LapEngineConfig, ReferenceMode};
use lmu_telemetry::{format_sample_line, SharedMemoryTelemetrySource, TelemetrySource};
use log::{info, warn};
use overlay_renderer::{config::OverlayConfig, TelemetryOverlay};
use storage::{ReferenceLapKey, ReferenceLapStore};
use telemetry_engine::TelemetrySnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cli {
    overlay: bool,
    configure: bool,
    print_config_path: bool,
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

    if cli.overlay {
        return run_overlay(cli.config_path);
    }

    let mut source = SharedMemoryTelemetrySource::open()?;

    if !source.is_available() {
        warn!(
            "LMU telemetry buffer is not available. Start LMU with built-in shared memory enabled."
        );
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
            configure: false,
            print_config_path: false,
            config_path: None,
            once: false,
            wait: false,
            interval: Duration::from_millis(100),
        };

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--overlay" => cli.overlay = true,
                "--configure" => cli.configure = true,
                "--print-config-path" => cli.print_config_path = true,
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
  hashoverlay [--overlay] [--configure] [--config <path>] [--once] [--wait] [--interval-ms <milliseconds>]

Options:
  --overlay                 Open the transparent always-on-top telemetry overlay.
  --configure               Create and open the user overlay config.
  --print-config-path       Print the default overlay config path.
  --config <path>           Use a custom overlay config path.
  --once                    Print one telemetry sample and exit after telemetry is detected.
  --wait                    Keep waiting when LMU telemetry is not available yet.
  --interval-ms <value>     Poll interval for CLI logging. Default: 100.
  -h, --help                Show this help."
    );
}

fn run_overlay(config_path: Option<PathBuf>) -> Result<()> {
    let mut source = SharedMemoryTelemetrySource::open()?;
    let config_path = overlay_config_path(config_path);
    OverlayConfig::save_default(&config_path)?;
    let config = OverlayConfig::load(&config_path)?;
    let lap_config = lap_engine_config(&config);
    let overlay = TelemetryOverlay::with_config(config)?;
    let lap_store = ReferenceLapStore::appdata();
    let lap_key = ReferenceLapKey::fallback();
    let personal_best = match lap_store.load_personal_best(&lap_key) {
        Ok(personal_best) => personal_best,
        Err(error) => {
            warn!("Could not load personal best reference lap: {error}");
            None
        }
    };
    let mut lap_engine = LapEngine::new(lap_config).with_personal_best(personal_best);
    let (lap_writer, lap_receiver) = mpsc::channel();
    thread::spawn(move || {
        while let Ok(lap) = lap_receiver.recv() {
            if let Err(error) = lap_store.save_personal_best(&lap_key, &lap) {
                warn!("Could not save personal best reference lap: {error}");
            }
        }
    });

    overlay.run(move || match source.read_sample() {
        Ok(Some(sample)) => {
            let mut snapshot = TelemetrySnapshot::from(sample);
            lap_engine.update(snapshot).apply_to(&mut snapshot);
            if let Some(lap) = lap_engine.take_new_personal_best() {
                let _ = lap_writer.send(lap);
            }
            Some(snapshot)
        }
        Ok(None) => None,
        Err(error) => {
            warn!("Could not read telemetry sample: {error}");
            None
        }
    })?;

    Ok(())
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
        ..LapEngineConfig::default()
    }
}

fn open_overlay_config(config_path: Option<PathBuf>) -> Result<()> {
    let config_path = overlay_config_path(config_path);
    OverlayConfig::save_default(&config_path)?;
    println!("Opening {}", config_path.display());
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
}
