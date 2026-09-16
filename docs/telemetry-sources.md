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

The host also fingerprints the verified raw frame to observe producer
progress. This avoids treating an invalid or unavailable session elapsed time
as proof that LMU has stopped producing telemetry. If the fingerprint remains
unchanged beyond the stale timeout, the reader reopens the mapping and waits
for a fresh frame.

The source layout is based on the `SharedMemoryInterface` header shipped with LMU under the game's `Support\SharedMemoryInterface` folder. TinyPedal's open source `pyLMUSharedMemory` project follows the same source and is useful as a reference implementation.

The generic `gameVersion` value is recorded by the interface but is not a
shared-memory layout revision. The reader therefore does not treat an
arbitrary version range as proof of compatibility: it validates the mapped
buffer size and marker bounds for the compiled layout and rejects malformed
markers. A future incompatible layout must receive its own verified parser.

Lap validity is intentionally conservative. When the active `LMU_Data` layout contains the verified `mLapInvalidated` field, the reader exposes `Some(true)` or `Some(false)`. If that field is unavailable in the installed layout, it remains `None`; HashOverlay never invents an offset or treats hidden process memory as a source of truth.

The scoring field is refreshed independently at a bounded rate (about 10 Hz), while player telemetry remains on the fast path. Each field row is copied into an immutable shared snapshot so widgets do not re-read or parse the 100+ vehicle entries on every render frame.

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
