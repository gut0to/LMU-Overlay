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
npm audit --audit-level=moderate
npm run build
npm run tauri build
```

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
