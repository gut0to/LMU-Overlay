# LMU Overlay

HashOverlay LMU is an open source, local-first telemetry overlay for **Le Mans Ultimate**.

The goal is simple: show useful live telemetry and delta information with very low overhead, while keeping the project fair-play friendly and easy to reason about.

## Project Status

This repository is in the first milestone. It currently contains:

- a read-only reader for LMU's built-in shared memory interface;
- a compact telemetry sample model;
- a small CLI that prints speed, gear, throttle, brake, RPM, lap and sector;
- a transparent always-on-top telemetry overlay window;
- a basic telemetry widget with input bars, steering bar, speed, gear, RPM and throttle/brake history;
- a user config opened from the app for position, size, opacity, colors and visible widgets;
- lap/reference logic for session best, personal best, live delta, predicted lap and mini-sectors;
- local binary PB storage under `%APPDATA%\HashOverlay\laps`;
- a Tauri + React + TypeScript settings app with presets and visual controls;
- a pure telemetry engine crate with a tested ring buffer;
- hotkeys for show/hide and edit mode;
- a debug performance monitor.

Packaging and deeper renderer profiling are still being refined.

## Principles

- 100% local.
- Read-only telemetry access.
- No DLL injection.
- No input automation.
- No hidden opponent data.
- No backend server, cloud dependency or database.
- Stable `main` branch.
- Small, focused pull requests.

## Quick Start

See [docs/usage.md](docs/usage.md) for the full step-by-step guide.

```powershell
cargo run -p hashoverlay -- --once
```

```powershell
cargo run -p hashoverlay -- --overlay
```

Open the overlay configuration:

```powershell
cargo run -p hashoverlay -- --configure
```

Run the settings app:

```powershell
cd settings
npm install
npm run tauri dev
```

If LMU is not running or its built-in shared memory interface is unavailable, the command exits cleanly with a warning.

## Development

```powershell
cargo test
```

```powershell
cargo run -p hashoverlay
```

Pull requests run the Windows Rust CI workflow with formatting, Clippy and tests.

## Architecture

The intended runtime flow is:

```text
LMU
  -> Built-in Shared Memory (LMU_Data)
  -> Telemetry Reader
  -> Telemetry Engine
  -> Snapshot
  -> Overlay Renderer
```

The renderer must not read shared memory directly. It will consume compact snapshots from the telemetry pipeline.

More detail is available in [docs/architecture.md](docs/architecture.md).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

This project is licensed under the [MIT License](LICENSE).
