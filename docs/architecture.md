# Architecture

```text
LMU
  -> Built-in Shared Memory (LMU_Data)
  -> lmu-telemetry
  -> telemetry-engine
  -> snapshot
  -> overlay renderer
```

The renderer must never read shared memory directly. It will consume compact snapshots produced by the telemetry pipeline.

## Milestone Order

1. Workspace, LMU detection, telemetry sample logging, tests.
2. Transparent overlay window.
3. Telemetry widget and input graphs.
4. Lap tracking, reference lap and live delta.
5. Delta widget, PB/session best, sectors and mini-sectors.
6. Ghost telemetry and coaching comparisons.
7. Tauri settings app, edit mode, hotkeys and presets.
8. Profiling, packaging and polish.

## Shared Memory Reader

The current reader opens LMU's built-in `LMU_Data` shared memory interface with `OpenFileMappingW` and `FILE_MAP_READ`. It does not call `CreateFileMapping`, because creating an empty buffer would make LMU detection unreliable and would violate the read-only intent.

The rFactor 2 shared memory plugin path is considered legacy/fallback for LMU and should not be the default Windows path.

## Renderer

The current renderer milestone creates a lightweight Win32 transparent, always-on-top, click-through window and draws the first telemetry widget. It is deliberately fixed-position for now; edit mode, resize handles and saved layout belong to the settings/edit-mode milestone.

## Open Source Direction

The repository is intended to stay approachable for contributors. Core telemetry calculation should remain isolated from Win32 integration so it can be tested without LMU running.

Public APIs should be boring and explicit. Avoid clever abstractions until there is repeated pressure from real milestones.
