# HashOverlay Settings design system

## Direction

The Settings surface uses a paddock timecard language: an operational sheet for a race engineer rather than a generic dashboard. It is deliberately bright for daylight and second-screen use, with strong ink lines and compact technical labeling.

## Tokens

- Ground: warm paper `#f3efe4` and bright paper `#fffdf6`.
- Structure: ink `#10233f`, cobalt `#185adb`, and slate `#bfd1d8`.
- State: acid lime `#c8f22d` for selected/live state; coral `#f05c4b` for destructive actions.
- Type: Archivo for UI language; DM Mono only for paths, labels, states, and measurements.

## Layout and interaction

- The header acts as a control ticket: identity, host state, and primary actions are visible without scrolling.
- Scene and surface selection are separate operational bands.
- The summary strip describes the selected surface before its settings panels.
- Panels retain their functional grouping and use a strict square-edge, paper-and-ink system.
- The only ambient motion is the live-state pulse, removed for reduced-motion users.

## Accessibility

Interactive controls preserve native semantics and strong focus rings. Selected, disabled, and destructive states use color plus text, border, or weight changes.
