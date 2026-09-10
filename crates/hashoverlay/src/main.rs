use std::{thread, time::Duration};

use anyhow::Result;
use lmu_telemetry::{format_sample_line, SharedMemoryTelemetrySource, TelemetrySource};
use log::{info, warn};

fn main() -> Result<()> {
    env_logger::init();

    let once = std::env::args().any(|arg| arg == "--once");
    let mut source = SharedMemoryTelemetrySource::open()?;

    if !source.is_available() {
        warn!("LMU telemetry buffer is not available. Start LMU with the shared memory plugin enabled.");
        return Ok(());
    }

    info!("LMU telemetry buffer detected.");

    loop {
        match source.read_sample() {
            Ok(Some(sample)) => println!("{}", format_sample_line(&sample)),
            Ok(None) => warn!("Telemetry buffer is present, but no player sample is available yet."),
            Err(error) => warn!("Could not read telemetry sample: {error}"),
        }

        if once {
            break;
        }

        thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

