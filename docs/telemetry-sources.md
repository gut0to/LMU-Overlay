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

The source layout is based on the `SharedMemoryInterface` header shipped with LMU under the game's `Support\SharedMemoryInterface` folder. TinyPedal's open source `pyLMUSharedMemory` project follows the same source and is useful as a reference implementation.

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
