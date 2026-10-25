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

The host supports F1–F12 with optional Ctrl, Shift and Alt modifiers. Other keys
leave the current binding unchanged. Conflict detection treats modifier order
and the Control/Ctrl spelling as equivalent.

While capturing a shortcut, Tab moves to the next control and Escape leaves the
shortcut unchanged. Modifier keys alone do not replace the shortcut.

## Finding Widgets and Recovering the Editor

The widget search matches names, identifiers, descriptions and telemetry
requirements. Combine it with a category filter; **Clear filters** resets both.
The result count and empty-state message help explain a filtered catalog.

Search also matches categories and availability status. Enter several words in
any order to require all of them, such as `lap fuel`; accents are optional. The
layout picker uses the same matching rules for widget names and identifiers.

Custom preset names keep focus while editing and use a fallback name if left
blank. Deleting a custom preset asks for confirmation.

The editor restores valid workspace dimensions and monitor origins from local
preferences. Corrupt preferences recover safe defaults, and unavailable browser
storage does not prevent editing. Use **Reset editor** to return to the default
workspace. Negative origins remain supported for monitors left of the primary.

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

## Saving and Recovery

Settings queues saves, reloads, imports and defaults resets in order. Edits made
while a save is running remain in the editor and are saved after that write
finishes. Failed autosaves show a retry message instead of retrying continuously;
use **Save** to retry or reload after resolving an external configuration change.

Ctrl+S (or Cmd+S in a browser preview) saves the current settings. Starting the
overlay requires a successful save. Reloading, importing or resetting settings
temporarily pauses editor interactions while the replacement is loaded.

Browser environments that support `beforeunload` warn about unsaved edits when
leaving the page. Confirm the save indicator before closing the native app.

## Precise Layout Editing

Widget snapping selects the closest alignment across visible widgets; equal
distances resolve consistently regardless of catalog order. A snap distance of
zero permits exact alignments only. Tiny overlay windows also constrain widget
scale so the widget remains inside the surface.

Dragging and resizing work with scaled widgets. Escape stops an active drag at
its current position; losing pointer capture also ends it. Locked widgets and
the global layout lock disable geometry controls in the inspector. Unlock the
widget, or turn off the global lock, before editing its dimensions.

**Reset size** restores the selected widget's default dimensions. **Fit surfaces**
shrinks windows larger than the workspace before moving them inside its bounds.

## Reference Lap Files

Personal best references live in `%APPDATA%\HashOverlay\laps`. HashOverlay keeps
reading the older reference format and legacy filenames. A current reference
takes priority over a legacy file for the same track and car.

An invalid reference produces a load warning and is ignored; the reader does not
rewrite it or silently substitute an older lap. To recover, close the overlay,
back up the affected `.pb-lap` file outside the laps folder, then record a new
valid lap. Invalid telemetry cannot overwrite a valid reference already saved.

## Undo and Redo

**Undo** and **Redo** beside Save restore recent configuration edits. Ctrl+Z
undoes; Ctrl+Y or Ctrl+Shift+Z redoes. On macOS browser previews, Cmd+Z and
Cmd+Shift+Z also work. Text fields keep their native undo behavior.

Each widget drag, resize or surface drag is one action. Release the pointer or
press Escape before using history. Restored edits autosave normally. The editor
keeps the most recent 40 actions in memory; closing Settings, reloading,
importing or resetting the configuration clears history. A new edit after Undo
discards the abandoned redo branch.

If saving succeeds but the running overlay cannot reload, Settings retains the
saved revision and reports the reload warning. Resolve the host problem and
press Save to retry applying it. Starting the overlay waits for a successful
save and live reload result.

## Saved Setups and Widget Selections

In **Presets**, use **New from current** to capture the selected surface's widget
positions and visibility together with the shared appearance, units, timing and
coaching settings. Custom presets preserve saved positions when applied;
standard presets arrange the widgets automatically. Window placement and hotkeys
are not part of a custom preset. Shared settings can affect all surfaces.

Search presets by name, duplicate a setup before experimenting, or move entries
up and down to choose their order in the setup selector. The list supports 32
custom presets. Replacing a saved setup or deleting one asks for confirmation.
The highlighted custom preset matches the current selected surface's profile.

In **Widgets**, combine search, category, visibility and availability filters.
**Enable matching** and **Disable matching** change the available results on the
selected surface in a single undoable action. Widgets assigned to other surfaces
remain enabled there. Unavailable LMU widgets remain excluded from bulk actions.

## Recovering an Interrupted Editing Session

Pending changes have a local recovery draft in the Settings webview. On reopening,
choose **Restore draft for review** or **Discard draft** before editing. A warning
identifies drafts created before an external change to the configuration file.
Restoring validates the candidate and keeps automatic saving paused; review it
and choose **Save** to apply it. Undo can return to the loaded configuration.
Reload, import or reset replaces the recovered editor state.

Drafts belong to one configuration path, expire after 30 days and have a 4 MiB
character limit. They are local convenience copies, not exported backups. Storage
errors do not block editing or saving, but Settings warns when recovery cannot be
kept. Export important setups before closing if saving or local storage fails.

## Review TOML Before Importing

In **Advanced**, paste TOML and choose **Preview import**. Settings validates and
migrates it without changing the configuration file or contacting the overlay.
The review lists the setting path, current value and imported value. Large reviews
show the first 80 changed fields and the full change count. Text edits invalidate
the preview, so **Apply reviewed import** always uses the text you reviewed.

An import replaces the entire configuration, including surfaces and custom
presets. Export first to keep a backup; **Copy TOML** copies the text for saving
elsewhere. Validation errors appear beside the editor. Empty or larger-than-4-MiB
imports are rejected, and an import identical to current settings needs no apply.

## Arrange Widgets Precisely

In the layout inspector, choose another visible widget as a reference and align
edges or centers. **Match reference size** matches its visible dimensions even
when the two widgets use different scales. The reference itself never moves.

**Copy layout** keeps geometry inside Settings. Select a different widget or
surface, then paste position, size or full geometry. Paste preserves the target's
opacity, lock state and stacking order; geometry is constrained to its surface.
The internal layout clipboard lasts only until Settings closes.

Use the movement controls for 1-pixel or 10-pixel adjustments. Arrow keys on a
focused preview widget move 1 pixel, or 10 with Shift. These precision operations
ignore snapping; dragging continues to use the snap settings. A held arrow key
is one undo action after release. Locked widgets and the global edit lock prevent
placement changes.

**Bring to front** and **Send to back** change the widget's stacking order within
native limits. **Fit window to widgets** resizes only the selected surface to its
visible content, without moving its screen origin or its widgets. Widget edits
belong to their selected surface and leave other surfaces' inherited templates
unchanged.
