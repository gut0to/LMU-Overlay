# Architecture

```text
LMU
  -> Shared Memory
  -> lmu-telemetry
  -> telemetry-engine
  -> snapshot
  -> overlay renderer
```

The renderer must never read shared memory directly. It will consume compact snapshots produced by the telemetry pipeline.

## Milestone Order

1. Workspace, LMU detection, telemetry sample logging, tests.
2. Transparent Direct2D window.
3. Telemetry widget and input graphs.
4. Lap tracking, reference lap and live delta.
5. Delta widget, PB/session best, sectors and mini-sectors.
6. Ghost telemetry and coaching comparisons.
7. Tauri settings app, edit mode, hotkeys and presets.
8. Profiling, packaging and polish.

## Shared Memory Reader

The current reader opens `$rFactor2SMMP_Telemetry$` with `OpenFileMappingW` and `FILE_MAP_READ`. It does not call `CreateFileMapping`, because creating an empty buffer would make LMU detection unreliable and would violate the read-only intent.

