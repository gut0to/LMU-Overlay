# Usage Guide

This guide explains how to clone, build and run the current milestone of LMU Overlay.

## 1. Install Requirements

Install:

- Windows 10 or newer;
- Git;
- Rust stable from <https://rustup.rs/>.

After installing Rust, open a new PowerShell window and check:

```powershell
rustc --version
cargo --version
```

## 2. Clone the Repository

```powershell
git clone https://github.com/gut0to/LMU-Overlay.git
cd LMU-Overlay
```

If you already cloned it:

```powershell
git pull
```

## 3. Build

```powershell
cargo build
```

## 4. Run Tests

```powershell
cargo test
```

For the same checks used by CI:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 5. Build A Release Package

Maintainers can publish a Windows release by pushing a semantic version tag:

```powershell
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds the release ZIP with:

- `hashoverlay.exe`;
- README, license and usage guide;
- the Settings app bundle.

## 6. Run One Telemetry Read

Start Le Mans Ultimate and enable plugins in the game settings:

```text
Settings -> Gameplay -> Enable Plugins -> On
```

LMU Overlay reads LMU's built-in `LMU_Data` shared memory interface on Windows. It does not require the rFactor 2 shared memory plugin for LMU.

Then run:

```powershell
cargo run -p hashoverlay -- --once
```

Expected behavior:

- If telemetry is available, the app prints speed, gear, throttle, brake, RPM, lap and sector.
- If telemetry is not available, the app exits cleanly with a warning.

## 7. Run Continuous Logging

```powershell
cargo run -p hashoverlay
```

Press `Ctrl+C` to stop.

To wait for LMU if it is not open yet:

```powershell
cargo run -p hashoverlay -- --wait
```

To change the CLI logging interval:

```powershell
cargo run -p hashoverlay -- --interval-ms 50
```

## 8. Configure The Overlay

For the full settings app:

```powershell
cd settings
npm install
npm run tauri dev
```

The settings app can change:

- presets for Practice, Qualifying and Race;
- visible widgets;
- position, size, scale and opacity;
- per-widget position, size, lock state and snap behavior;
- live layout preview with draggable/resizable mock widget boxes;
- colors and line thickness;
- font family, font size, large-number size and km/h or mph units;
- telemetry history, render FPS and sample interval;
- delta reference, mini-sectors and coaching thresholds;
- hotkeys and performance mode.

For a lightweight text config fallback:

```powershell
cargo run -p hashoverlay -- --configure
```

This creates and opens:

```text
%APPDATA%\HashOverlay\hashoverlay.toml
```

You can edit the same settings in TOML. Save the file, then start the overlay.
When the overlay is already running, saved settings are hot reloaded automatically.

## 9. Run The Overlay

Start LMU, enter a session, and run:

```powershell
cargo run -p hashoverlay -- --overlay
```

The current overlay includes:

- transparent;
- always on top;
- click-through;
- configurable position and size;
- independent widget position and size;
- configurable scale, opacity, colors and line thickness;
- toggles for title, speed/RPM, pedals, steering, lap info and input history;
- speed, gear and RPM text;
- throttle, brake and clutch bars;
- steering bar;
- throttle and brake history graphs.
- live delta once a reference lap exists;
- predicted lap, personal best, session best and mini-sector indicator;
- mini-sector delta based on the last completed mini-sector crossing;
- ghost input markers for throttle/brake when a reference lap exists;
- brake/throttle timing hints against the reference lap;
- coaching cues for releasing brake, adding throttle and carrying more speed.
- F9 show/hide by default;
- F10 edit mode by default;
- drag-to-move and corner resize for each unlocked widget while edit mode is enabled;
- persistent layout saves when a widget is moved or resized;
- hot reload when the Settings app saves config changes.

## 10. Reference Laps And PB Storage

HashOverlay stores personal-best reference laps under:

```text
%APPDATA%\HashOverlay\laps
```

Reference laps are keyed by LMU telemetry metadata, using track, vehicle and class. That keeps a Sebring/Porsche PB separate from a Le Mans/Ferrari PB.

Only clean green-flag samples outside the pits and garage are eligible for session-best and personal-best references. Pit/garage/out-of-session samples can still be shown by the overlay, but they do not overwrite your reference laps.

## 11. Troubleshooting

If PowerShell says `cargo` is not recognized, Rust is not installed or the terminal was opened before Rust updated the PATH.

If the app says the telemetry buffer is not available:

- start LMU before running the command;
- confirm `Enable Plugins` is turned on in LMU's gameplay settings;
- run the terminal as the same Windows user that is running the game.

## 12. Current Limitations

- The renderer still uses the current Win32/GDI backend while the Direct2D/DirectWrite backend is prepared.
- LMU lap invalidation by track-cut is not mapped until an official field is identified in `LMU_Data`; the current validity gate is conservative around green flag, pits and garage.
