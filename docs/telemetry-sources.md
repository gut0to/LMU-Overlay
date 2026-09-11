# Telemetry Sources

LMU Overlay should prefer telemetry sources in this order:

1. LMU built-in shared memory: `LMU_Data`.
2. LMU Rest API for slower supplemental data.
3. rFactor 2 shared memory plugin only as a legacy/fallback path, if explicitly added later.

## LMU Built-In Shared Memory

On Windows, Le Mans Ultimate exposes a built-in shared memory interface named `LMU_Data`.

This is the primary source for hot-path overlay telemetry:

- speed;
- RPM;
- gear;
- throttle;
- brake;
- clutch;
- steering;
- sector;
- lap number;
- lap distance;
- track length.

The reader also extracts session metadata from the same official buffer:

- track name;
- vehicle name and class;
- LMU session kind;
- game phase;
- player slot ID;
- pit and garage state.

To avoid reading a half-updated frame, the reader captures a small frame marker before and after the sample. If the marker changes during the read, the sample is retried and then rejected as a torn frame. This keeps the overlay conservative without touching process memory.

The source layout is based on the `SharedMemoryInterface` header shipped with LMU under the game's `Support\SharedMemoryInterface` folder. TinyPedal's open source `pyLMUSharedMemory` project follows the same source and is useful as a reference implementation.

Lap validity is intentionally conservative. The current mapped `LMU_Data` fields identify green-flag state, pits and garage, but do not yet expose a confirmed official track-cut/lap-invalidated flag in this reader. Until that field is mapped from the official header, HashOverlay will not invent an offset or treat hidden process memory as a source of truth.

## LMU Rest API

LMU also exposes a local Rest API, usually on:

```text
localhost:6397
```

This should not be used for the high-frequency telemetry path. Use it later for lower-frequency supplemental data such as vehicle metadata, weather, garage/setup information, pit timing, damage or brake wear.

## Fair Play Boundary

Reading `LMU_Data` is not process memory scanning and does not write into the game. The overlay must continue to avoid:

- `ReadProcessMemory`;
- writing game memory;
- DLL injection;
- input automation;
- hidden opponent data;
- bypassing game protections.
