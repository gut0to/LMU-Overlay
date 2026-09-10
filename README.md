# LMU Overlay

LMU Overlay is an open source, local-first telemetry overlay for **Le Mans Ultimate**.

The goal is simple: show useful live telemetry and delta information with very low overhead, while keeping the project fair-play friendly and easy to reason about.

## Project Status

This repository is in the first milestone. It currently contains:

- a Rust workspace;
- a read-only shared memory telemetry reader;
- a compact telemetry sample model;
- a small CLI that prints speed, gear, throttle, brake, RPM, lap and sector;
- a pure telemetry engine crate with a tested ring buffer;
- architecture notes for the future overlay renderer and settings app.

The visual overlay, Direct2D renderer and Tauri settings app are not implemented yet.

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

If LMU or the shared memory plugin is not running, the command exits cleanly with a warning.

## Development

```powershell
cargo test
```

```powershell
cargo run -p hashoverlay
```

## Architecture

The intended runtime flow is:

```text
LMU
  -> Shared Memory
  -> Telemetry Reader
  -> Telemetry Engine
  -> Snapshot
  -> Direct2D Renderer
```

The renderer must not read shared memory directly. It will consume compact snapshots from the telemetry pipeline.

More detail is available in [docs/architecture.md](docs/architecture.md).

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

This project is licensed under the [MIT License](LICENSE).
