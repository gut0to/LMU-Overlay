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

Do not add LMU offsets without verifying the `SharedMemoryInterface` header distributed with the game. Keep the reader read-only and add coverage for every conversion or derived calculation.
