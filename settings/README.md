# HashOverlay Settings

Tauri + React + TypeScript control room for HashOverlay. It loads the widget
catalog, edits local TOML configuration, starts or stops the single overlay
host and reports its ready state through the local host-control pipe.

## Development

```powershell
cd settings
npm ci
npm run tauri dev
```

The app edits the same local config used by the runtime overlay:

```text
%APPDATA%\HashOverlay\hashoverlay.toml
```

The overlay keeps working without this app. It loads the config file when
started. Settings reports the host as live only after its overlay surface and
control service are ready.

## Validation

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```
