# HashOverlay Settings

Tauri + React + TypeScript settings panel for HashOverlay.

## Development

```powershell
cd settings
npm install
npm run tauri dev
```

The app edits the same local config used by the runtime overlay:

```text
%APPDATA%\HashOverlay\hashoverlay.toml
```

The overlay keeps working without this app. It loads the config file when started.
