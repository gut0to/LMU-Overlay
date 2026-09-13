# Using HashOverlay

## Install and Drive

1. Download the Windows release ZIP and extract it anywhere you can write to.
2. Open `HashOverlay Settings.exe`.
3. Select **Practice**, **Qualifying** or **Race**, then open **Widgets** to choose what is visible.
4. Use **Layout** to drag, resize, lock and snap widgets in the preview. Press **Save**.
5. Select **Start overlay**.
6. Start Le Mans Ultimate, enable its built-in plugin/shared-memory option, and enter a session.

The overlay displays **WAITING FOR LMU** until it has a valid player sample. It never manufactures values while LMU is closed.

## In-Game Controls

- `F9`: show or hide the overlay.
- `F10`: toggle edit mode. In edit mode the overlay stops click-through so unlocked widgets can be moved and resized. Press `F10` again to race normally.
- `Shift+F10`: toggle coaching.
- `Ctrl+Shift+F9`: cycle the saved profiles.

All shortcuts can be changed in **Settings > Hotkeys**. Conflicting shortcuts are highlighted before saving.

## Choosing a Layout

The default layout is deliberately small: driving HUD, inputs, timing, relative/race information when available, fuel and flags. Turn on additional widgets one by one rather than placing everything on screen.

Use the Widget Browser to search by name or filter by category. A widget can be enabled even if LMU does not currently expose its official data on your installed game version; it will show `--` until that data becomes available.

## Settings and Storage

Settings are stored in `%APPDATA%\HashOverlay\hashoverlay.toml`. You normally never need to edit it. The app supports export, import, reset, live preview and hot reload while the overlay is running.

Personal-best reference laps are stored under `%APPDATA%\HashOverlay\laps` and keyed by track, track layout and car. Only valid non-pit laps can update a PB or coaching reference.

## Troubleshooting

If the overlay stays on **WAITING FOR LMU**, start LMU first, confirm its shared-memory/plugins option is enabled, and make sure the game and HashOverlay run as the same Windows user.

For source-level development and validation, see [development.md](development.md). For the exact data-source limits, see [telemetry-sources.md](telemetry-sources.md).
