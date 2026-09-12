# Architecture

```text
LMU
  -> Built-in Shared Memory (LMU_Data)
  -> lmu-telemetry
  -> telemetry-engine snapshots
  -> lap-engine
  -> widget catalog and renderer
```

The renderer must never read shared memory directly. It will consume compact snapshots produced by the telemetry pipeline.

The telemetry path is intentionally split by rate: fast controls remain on the acquisition path, while session/scoring additions can update at a lower rate. `TelemetrySnapshot` carries optional vehicle, wheel and session values so a missing official LMU field becomes an explicit unavailable state instead of a fabricated zero.

## Shared Memory Reader

The current reader opens LMU's built-in `LMU_Data` shared memory interface with `OpenFileMappingW` and `FILE_MAP_READ`. It does not call `CreateFileMapping`, because creating an empty buffer would make LMU detection unreliable and would violate the read-only intent.

The rFactor 2 shared memory plugin path is considered legacy/fallback for LMU and should not be the default Windows path.

## Renderer

The current renderer creates a lightweight Win32 transparent, always-on-top, click-through window and draws telemetry, input, timing, coaching, engineering and performance widgets. The central widget catalog supplies stable IDs, categories, descriptions and data requirements to the Settings app. Window placement, layout, scale, opacity, colors, refresh rate, performance mode, hotkeys and visibility are loaded from one config file.

F9 toggles visibility by default. F10 toggles edit mode by default, which makes the overlay clickable so each unlocked widget can be selected, moved, resized and saved back to the config file.

Saved config changes are hot reloaded while the overlay is running. This lets the Settings app change colors, layout, opacity, units, coaching and timing behavior without restarting the overlay.

The renderer still uses GDI drawing while the Direct2D/DirectComposition backend is being refined. The module boundary keeps that swap isolated from telemetry and lap logic.

## Lap And Delta Engine

`lap-engine` owns reference laps, session best, personal best, live delta, predicted lap, mini-sectors and initial coaching comparisons. It operates on compact telemetry snapshots and does not know anything about Win32, shared memory or rendering.

Reference lap lookup is progress-based, not timestamp-based. The hot-path lookup samples a sorted reference lap by normalized track progress and interpolates between nearby points.

Personal best laps are written by a background thread through `storage`, outside the telemetry/render loop. Reference lap files are keyed from LMU telemetry metadata using track, track layout and car; early V3 keys remain readable for compatibility.

## Open Source Direction

The repository is intended to stay approachable for contributors. Core telemetry calculation should remain isolated from Win32 integration so it can be tested without LMU running.

Public APIs should be boring and explicit. Avoid clever abstractions until there is repeated pressure from real milestones.

## Settings App

`settings/` contains the Tauri + React + TypeScript overlay designer. It edits the same TOML config consumed by the runtime overlay, loads the widget catalog from Rust, starts an adjacent release overlay executable, opens the config folder, provides a searchable widget browser, live layout preview, import/export and reset actions. Normal use does not require hand-editing TOML.

## CI

Pull requests run two Windows jobs:

- Rust formatting, Clippy and workspace tests.
- Settings dependency install, npm audit, frontend build and Tauri shell check.
