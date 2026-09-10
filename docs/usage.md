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

## 5. Run One Telemetry Read

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

## 6. Run Continuous Logging

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

## 7. Troubleshooting

If PowerShell says `cargo` is not recognized, Rust is not installed or the terminal was opened before Rust updated the PATH.

If the app says the telemetry buffer is not available:

- start LMU before running the command;
- confirm `Enable Plugins` is turned on in LMU's gameplay settings;
- run the terminal as the same Windows user that is running the game.

## Current Limitations

- The Direct2D overlay window is not implemented yet.
- Delta, predicted lap, PB and ghost telemetry are planned for later milestones.
- The settings app is planned for the Tauri milestone.
