# Development

HashOverlay is a Rust workspace with a Tauri/React Settings app. LMU is not required for pure unit tests.

## Rust

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

## Settings

```powershell
cd settings
npm ci
npm test
npm audit --audit-level=moderate
npm run build
npm run tauri build
```

The Settings tests require Node.js 22.6 or newer and validate recovery of editor
workspace preferences, shortcut capture, search, configuration queue recovery,
save acknowledgements, shared host polling, numeric limits, widget normalization
and snapping. Delayed-operation tests verify ordering and preservation of newer
edits. They do not require the Tauri host.

The frontend build checks TypeScript and produces the Vite bundle. The Tauri
build packages that bundle with the matching `hashoverlay.exe` resource. Run
the following Rust checks for the Settings shell when changing its commands,
startup behavior or bundled resources:

```powershell
cargo test --manifest-path settings/src-tauri/Cargo.toml
cargo clippy --manifest-path settings/src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Startup Smoke Test

On Windows, build the release overlay and start it with the same configuration
path that Settings uses. The expected lifecycle is: host control reports
`running`, one window exists for each enabled surface, and a `stop` command
returns `stopping` before the process exits. This test does not require LMU to
be running; live telemetry validation remains covered by
[live-smoke-test.md](live-smoke-test.md).

Do not add LMU offsets without verifying the `SharedMemoryInterface` header distributed with the game. Keep the reader read-only and add coverage for every conversion or derived calculation.

## Reference and Numeric Regression Tests

These checks run without a live LMU session:

```powershell
cargo test -p storage
cargo test -p lap-engine
cargo test -p lmu-telemetry sample::tests::
cargo test -p telemetry-engine
```

Storage regression fixtures use unique temporary directories and remove them on
drop. Tests cover invalid writes preserving existing references, V1 compatibility,
canonical/legacy priority, truncated and corrupt records, oversized files and
temporary-file cleanup after replacement failure. Numeric tests exercise invalid
steering, unavailable circuit geometry, reference queries and fuel overflows.

## Host Protocol and Settings History

```powershell
cargo test -p host-protocol
cargo test --manifest-path settings/src-tauri/Cargo.toml
cd settings
npm test
```

The protocol suite covers command round trips, malformed frames, reply states,
UTF-8 error truncation and native key bindings. Settings history tests exercise
undo/redo branches, grouped gestures, bounded retention, immutable reducers,
file replacement, delayed acknowledgements and native text shortcut ownership.
The shell tests verify that live-reload failure still returns the persisted
configuration and revision.
