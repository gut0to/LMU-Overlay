# HashOverlay LMU

HashOverlay is a local, read-only racing overlay for Le Mans Ultimate. It is built around the game's official shared-memory interface and keeps all telemetry, references and settings on your PC.

## Quick Start

1. Download and extract the Windows release.
2. Open **HashOverlay Settings**.
3. Choose a preset, adjust the widgets you want and press **Start overlay**.
4. Start Le Mans Ultimate and enter a session.

The overlay waits safely for LMU when the game is not running. By default, `F9` shows or hides it and `F10` enters layout edit mode.

The full guide is in [docs/usage.md](docs/usage.md). Widget availability and data sources are listed in [docs/widgets.md](docs/widgets.md).

## What It Does

- Transparent, always-on-top, click-through overlay with in-game drag, resize and snapping.
- Settings app with a widget catalog, search, category filters, presets, hotkeys, preview, import/export and hot reload.
- Core driving HUD: gear, speed, RPM, pedals, steering, input history, delta, timing, sectors and mini sectors.
- Local reference-lap analysis: PB/session references, predicted lap, bounded histories and coaching.
- Optional engineering and race widgets for fuel, tyres, brakes, electronics, energy, engine, weather, flags, position and scoring-derived views.
- Explicit unavailable states: data that LMU has not officially exposed is shown as `--`, never invented.

## Fair Play

HashOverlay opens `LMU_Data` with read-only access. It does not inject code, write game memory, automate input, alter vehicle behaviour, use a cloud backend or read hidden data. See [docs/telemetry-sources.md](docs/telemetry-sources.md) for the source boundary.

## Project Notes

The public data model and widget catalog are intentionally broader than the currently verified LMU header available on this machine. Fields that need confirmation from LMU's shipped `Support/SharedMemoryInterface` remain optional until that header is verified. This protects users from misleading readings while keeping the overlay ready to consume official additions.

Architecture, development and release notes live in [docs/architecture.md](docs/architecture.md), [docs/development.md](docs/development.md) and [docs/release.md](docs/release.md).

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

Licensed under the [MIT License](LICENSE).
