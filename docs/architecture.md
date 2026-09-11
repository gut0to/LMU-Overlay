# Architecture

```text
LMU
  -> Built-in Shared Memory (LMU_Data)
  -> lmu-telemetry
  -> telemetry-engine
  -> lap-engine
  -> snapshot
  -> overlay renderer
```

The renderer must never read shared memory directly. It will consume compact snapshots produced by the telemetry pipeline.

## Milestone Order

1. Workspace, LMU detection, telemetry sample logging, tests.
2. Transparent overlay window.
3. Telemetry widget and input graphs.
4. User-editable overlay configuration.
5. Lap tracking, reference lap and live delta.
6. Delta widget, PB/session best, sectors and mini-sectors.
7. Ghost telemetry and coaching comparisons.
8. Tauri settings app, hotkeys, presets and clickable edit mode.
9. Persistent widget drag/resize handles, profiling, packaging and polish.

## Shared Memory Reader

The current reader opens LMU's built-in `LMU_Data` shared memory interface with `OpenFileMappingW` and `FILE_MAP_READ`. It does not call `CreateFileMapping`, because creating an empty buffer would make LMU detection unreliable and would violate the read-only intent.

The rFactor 2 shared memory plugin path is considered legacy/fallback for LMU and should not be the default Windows path.

## Renderer

The current renderer creates a lightweight Win32 transparent, always-on-top, click-through window and draws telemetry, input, delta, coaching and performance widgets. Window placement, widget layout, scale, opacity, colors, refresh rate, performance mode, hotkeys and visible widgets are loaded from the user config file.

F9 toggles visibility by default. F10 toggles edit mode by default, which makes the overlay clickable so each unlocked widget can be selected, moved, resized and saved back to the config file.

Saved config changes are hot reloaded while the overlay is running. This lets the Settings app change colors, layout, opacity and timing behavior without restarting the overlay.

The renderer still uses GDI drawing while the Direct2D/DirectComposition backend is being refined. The module boundary keeps that swap isolated from telemetry and lap logic.

## Lap And Delta Engine

`lap-engine` owns reference laps, session best, personal best, live delta, predicted lap, mini-sectors and initial coaching comparisons. It operates on compact telemetry snapshots and does not know anything about Win32, shared memory or rendering.

Reference lap lookup is progress-based, not timestamp-based. The hot-path lookup samples a sorted reference lap by normalized track progress and interpolates between nearby points.

Personal best laps are written by a background thread through `storage`, outside the telemetry/render loop. Reference lap files are keyed from LMU telemetry metadata using track, vehicle and class.

## Open Source Direction

The repository is intended to stay approachable for contributors. Core telemetry calculation should remain isolated from Win32 integration so it can be tested without LMU running.

Public APIs should be boring and explicit. Avoid clever abstractions until there is repeated pressure from real milestones.

## Settings App

`settings/` contains the Tauri + React + TypeScript settings panel. It edits the same TOML config consumed by the runtime overlay and is not required while driving. Presets are stored structurally in the config so contributors can adjust Practice, Qualifying and Race defaults without touching the renderer. The app includes a live layout preview so users can position widgets without editing TOML.
