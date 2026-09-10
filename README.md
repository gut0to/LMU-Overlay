# HashOverlay

HashOverlay is a lightweight local overlay for Le Mans Ultimate telemetry.

The first milestone focuses on a stable Rust workspace and a read-only telemetry reader for the rFactor2SharedMemoryMapPlugin buffer used by LMU-compatible tools.

## Current Scope

- Rust workspace bootstrap.
- Read-only detection of the telemetry shared memory buffer.
- Compact telemetry sample model.
- Basic CLI that prints speed, gear, throttle, brake, RPM, lap and sector.
- Pure telemetry-engine ring buffer with tests.

## Fair Play

HashOverlay is read-only. It does not write LMU memory, inject DLLs, automate inputs, or use hidden opponent data.

## Run

```powershell
cargo run -p hashoverlay -- --once
```

If LMU or the shared memory plugin is not running, the command exits cleanly with a warning.

## Test

```powershell
cargo test
```

## Notes

LMU support is based on the rFactor2SharedMemoryMapPlugin telemetry buffer name `$rFactor2SMMP_Telemetry$`. If a future LMU build exposes an official native shared memory layout, this crate should add that reader behind the same `TelemetrySource` trait instead of changing renderer or engine code.

