# Settings

The current settings interface is the user config opened by:

```powershell
cargo run -p hashoverlay -- --configure
```

It creates `%APPDATA%\HashOverlay\hashoverlay.toml` with editable overlay position, size, opacity, colors, refresh rate, sample rate and widget toggles.

The richer settings app will be added later with Tauri, React and TypeScript. The runtime overlay must keep working when that app is closed or fails.
