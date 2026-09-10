use std::{process::ExitCode, thread, time::Duration};

use anyhow::Result;
use lmu_telemetry::{format_sample_line, SharedMemoryTelemetrySource, TelemetrySource};
use log::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cli {
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
            once: false,
            wait: false,
            interval: Duration::from_millis(100),
        };

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
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
  hashoverlay [--once] [--wait] [--interval-ms <milliseconds>]

Options:
  --once                    Print one telemetry sample and exit after telemetry is detected.
  --wait                    Keep waiting when LMU telemetry is not available yet.
  --interval-ms <value>     Poll interval for CLI logging. Default: 100.
  -h, --help                Show this help."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_cli() {
        let cli = Cli::parse(Vec::new());

        assert!(!cli.once);
        assert!(!cli.wait);
        assert_eq!(cli.interval, Duration::from_millis(100));
    }

    #[test]
    fn parses_cli_flags() {
        let cli = Cli::parse([
            "--once".to_string(),
            "--wait".to_string(),
            "--interval-ms".to_string(),
            "25".to_string(),
        ]);

        assert!(cli.once);
        assert!(cli.wait);
        assert_eq!(cli.interval, Duration::from_millis(25));
    }
}
