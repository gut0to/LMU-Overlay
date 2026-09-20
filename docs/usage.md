# Using HashOverlay

## Install and Drive

1. Download the Windows release ZIP and extract it anywhere you can write to.
2. Open `HashOverlay Settings.exe`.
3. Select **Practice**, **Qualifying** or **Race**, then open **Widgets** to choose what is visible.
4. Use **Layout** to choose a surface, position its window on the screen map, then drag, resize, lock and snap widgets in the preview. Changes autosave after a short pause; the save indicator shows when they are persisted.
5. Select **Start overlay** and wait for Settings to report **Overlay live**.
6. Start Le Mans Ultimate and enter a session. The overlay reads LMU's built-in `LMU_Data` interface; no external telemetry plugin is required.

The widget list and customization live in **HashOverlay Settings**, not inside LMU. LMU only provides the live data; use **Widgets**, **Layout** and **Appearance** in Settings, save, then start the overlay.

Settings only reports **Overlay live** after the overlay window and its local
control service are ready. The overlay displays **WAITING FOR LMU** until it
has a valid player sample. It never manufactures values while LMU is closed.

## In-Game Controls

- `F9`: show or hide the overlay.
- `F10`: toggle edit mode. In edit mode the overlay stops click-through so unlocked widgets can be moved and resized. Press `F10` again to race normally.
- `Shift+F10`: toggle coaching.
- `Ctrl+Shift+F9`: cycle the saved profiles.

All shortcuts can be changed in **Settings > Hotkeys**. Conflicting shortcuts are highlighted before saving.

## Surface and Window Editing

The surface map represents the selected monitor workspace. Click a surface to
select it, drag its rectangle to move the real overlay window, or focus it and
use the arrow keys for one-pixel movement. Hold `Shift` with an arrow for a
ten-pixel step. The inspector exposes exact screen X/Y, width and height.

The **Edit overlay** button in Settings enters the same native edit mode as
`F10`; **Exit edit mode** returns to click-through racing mode. The Settings
panel also provides **Show/Hide overlay** without stopping telemetry.

Workspace presets cover common 1080p, 1440p, ultrawide, 4K and extended-desktop
arrangements. Use the custom workspace fields for a different resolution or a
monitor whose Windows origin is negative. If a surface is outside the selected
workspace, use **Fit surfaces** to bring it back without changing its widgets.

Surface positions and sizes are persisted independently per surface. Duplicating
a surface copies its widget membership and layout overrides while offsetting the
new window so it remains visible during setup.

## Choosing a Layout

The default layout is deliberately small: driving HUD, inputs, timing, relative/race information when available, fuel and flags. Turn on additional widgets one by one rather than placing everything on screen.

If you are upgrading an older installation, select a preset and press **Save** once. Existing configuration is preserved during migration, so widgets that were previously disabled do not turn on automatically. The Settings installer includes the matching overlay executable; start the overlay from the same Settings installation so both use `%APPDATA%\\HashOverlay\\hashoverlay.toml`.

Use the Widget Browser to search by name or filter by category. A widget can be enabled even if LMU does not currently expose its official data on your installed game version; it will show `--` until that data becomes available.

## Settings and Storage

Settings are stored in `%APPDATA%\HashOverlay\hashoverlay.toml`. You normally never need to edit it. The app supports export, import, reset, live preview and hot reload while the overlay is running. If a save cannot be reloaded by the running host, Settings shows the reload error so you can correct it instead of assuming the new layout is live.

Personal-best reference laps are stored under `%APPDATA%\HashOverlay\laps` and keyed by track, track layout and car. Only valid non-pit laps can update a PB or coaching reference.

## Troubleshooting

If Settings does not reach **Overlay live**, close any stale HashOverlay process and start the overlay again from Settings. If the overlay stays on **WAITING FOR LMU**, start LMU first, enter a driving session, and make sure the game and HashOverlay run as the same Windows user. On older LMU builds, update the game so the built-in `LMU_Data` interface is available.

For source-level development and validation, see [development.md](development.md). For the exact data-source limits, see [telemetry-sources.md](telemetry-sources.md).
