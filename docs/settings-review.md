# Settings reliability review

The reviewed baseline is a Rust overlay workspace and React/Tauri Settings app
at version 1.0.5. Settings already supports multiple surfaces, live preview,
presets, telemetry availability labels and debounced persistence. The worktree
was clean and the remote default branch was `main`.

This change concentrates on Settings usability and preference recovery:

- Validate persisted workspace JSON without coercing booleans, strings or null
  into dimensions; bound sizes and preserve negative monitor origins.
- Keep editing and reset available when browser storage throws.
- Reject empty/nonfinite number edits and preserve keyboard traversal during
  hotkey capture.
- Name icon actions, announce save status and expose selection states for
  keyboard and assistive technology users.
- Search widget identifiers and telemetry requirements, show result counts and
  offer a filter reset and useful empty state.
- Improve field overflow, long names, touch targets, forced colors, reduced
  motion and narrow-screen action/filter layouts while retaining the palette.

Preference recovery has six regression tests with no new dependencies. Existing
Rust tests continue to cover telemetry conversion, lap references, layouts,
storage and host commands. Browser preference tests and frontend compilation
do not prove live LMU behavior or packaged Tauri behavior. Live driving and
assistive technology validation remain manual checks in the release process.
