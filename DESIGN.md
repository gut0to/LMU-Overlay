---
name: HashOverlay Settings
description: Local LMU overlay control room for configuring and operating race telemetry surfaces.
colors:
  primary: "#6df18b"
  primary-deep: "#1a9e54"
  neutral-void: "#07130e"
  neutral-rail: "#0b1d15"
  neutral-surface: "#0f271c"
  neutral-field: "#091912"
  neutral-ink: "#e5eee6"
  neutral-muted: "#91ab99"
  neutral-line: "#2a513c"
  danger: "#ff7a70"
typography:
  display:
    fontFamily: "Barlow Condensed, Arial Narrow, sans-serif"
    fontSize: "clamp(2rem, 4vw, 3.0625rem)"
    fontWeight: 700
    lineHeight: 0.84
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Manrope, Segoe UI Variable, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.45
  label:
    fontFamily: "Barlow Condensed, Arial Narrow, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    letterSpacing: "0.12em"
rounded:
  control: "3px"
spacing:
  compact: "7px"
  control: "12px"
  section: "24px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.neutral-void}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "36px"
  button-secondary:
    backgroundColor: "transparent"
    textColor: "{colors.neutral-ink}"
    rounded: "{rounded.control}"
    padding: "0 12px"
    height: "36px"
  input-control:
    backgroundColor: "{colors.neutral-field}"
    textColor: "{colors.neutral-ink}"
    rounded: "{rounded.control}"
    padding: "6px 9px"
    height: "34px"
---

# Design System: HashOverlay Settings

## Overview

**Creative North Star: "The Endurance Race Control"**

HashOverlay Settings is a quiet, low-light operations console for the interval between leaving the garage and taking the green flag. It is dense without becoming decorative: the selected overlay, its deployment state, controls, and live configuration are all available as one readable control surface.

The system refuses generic dashboard cards. Instead, thin green rules, continuous working panes, and an anchored left control rail organize the page. Green is an operational signal — reserve it for selection, readiness, focus and the committed action.

**Key Characteristics:**

- Dark trackside environment designed for second-screen use.
- Green communicates active control and ready state, never ambient decoration.
- Condensed display typography gives headings the decisiveness of pit equipment labeling.
- Flat, connected working surfaces keep dense configuration readable.

## Colors

The palette is an emerald signal system on near-black racing-garage surfaces, with warm off-white used only for legible working text.

### Primary

- **Signal Green:** the primary action, active tab, live indicator and keyboard focus color.
- **Staged Green:** the lower-luminance border and operational-ready color used before an action becomes primary.

### Secondary

- **Brake Amber:** reserved for attention states that are not destructive.
- **Pitlane Red:** destructive actions and configuration errors only.

### Neutral

- **Track Void:** the application ground and scrollbar track.
- **Control Rail:** the darkest raised region used for persistent navigation.
- **Working Surface:** continuous operational content regions.
- **Field Black:** editable inputs and overlay preview ground.
- **Helmet White:** primary reading color.
- **Telemetry Muted:** supporting copy and inactive controls.

**The Signal Budget Rule.** Green carries state and commitment. Do not use it as a generic decoration, broad background wash, or substitute for hierarchy.

## Typography

**Display Font:** Barlow Condensed, with Arial Narrow fallback.
**Body Font:** Manrope, with Segoe UI Variable fallback.
**Label Font:** Barlow Condensed for compact uppercase operational labels.

**Character:** Barlow Condensed gives section names and state readouts the footprint of physical cockpit controls. Manrope remains neutral and legible for dense configuration values and recovery guidance.

### Hierarchy

- **Display:** condensed bold uppercase, used for the app name only.
- **Headline:** condensed semibold uppercase, used for panel titles and primary metrics.
- **Title:** condensed semibold, used for active surface and configuration groups.
- **Body:** Manrope medium at 11–12px, used for controls, explanations and paths.
- **Label:** condensed uppercase with expanded tracking, used for state labels and compact metrics.

**The Control Label Rule.** A label must name a state, value, or action. Do not add decorative eyebrows above headings.

## Layout

The desktop surface uses a 222px left rail and a fluid operations area. The header spans the full frame; preset selection belongs in the rail, while surface selection, state metrics and configuration panels flow through the working area. Panels are adjacent operational regions separated by one-pixel rules, not floating cards.

At 1060px, the rail narrows and metrics use a two-column strip. At 800px, the layout becomes a single column: navigation turns horizontal and every configuration group stacks in reading order. The working rhythm is 7px for tight controls, 12px for control padding, and 24px for section separation.

## Elevation & Depth

Depth is primarily tonal: void, rail, surface and field are distinct luminance steps. The overlay preview alone uses a soft downward shadow because it represents a window sitting over the game; ordinary panels stay flat and are divided with rules.

**The One-Window Rule.** Soft shadow belongs only to the overlay preview or a genuinely elevated transient surface, never to every configuration group.

## Shapes

Controls use a restrained 3px radius. Borders are thin, dark green lines; selected controls become brighter through border and text color rather than larger geometry. Avoid pills, soft round cards, glass effects and decorative gradients.

## Components

### Buttons

- **Shape:** compact instrument controls with a 3px radius.
- **Primary:** Signal Green background with Track Void text for Start, Save and other committed actions.
- **Hover / Focus:** hover lifts by one pixel; keyboard focus uses a 2px Signal Green outline with a 3px offset.
- **Secondary / Ghost:** transparent, line-bounded controls that turn green on hover.

### Inputs / Fields

- **Style:** Field Black background, one-pixel neutral-line border and 3px radius.
- **Focus:** Signal Green outline, never a vague glow.
- **Error / Disabled:** Pitlane Red identifies an invalid field; disabled controls lower opacity without hiding their label.

### Navigation

- **Style:** a persistent vertical control rail on desktop, using condensed uppercase labels.
- **State:** the active section has a Signal Green left rule and a narrow green tint; hover is quieter and keeps the rail intact.
- **Mobile:** navigation becomes a horizontal, scrollable row with the selected state moved to the bottom edge.

### Surface Selector

- **Style:** working-window buttons with one-pixel borders and compact metadata.
- **State:** the selected surface uses Signal Green type and border on a subtle green-tinted ground.
- **Role:** it is the operational handoff between a multi-window overlay runtime and the configuration form.

## Do's and Don'ts

### Do:

- **Do** use Signal Green only for live, selected, focused and primary-action states.
- **Do** keep high-density controls in continuous panes separated by one-pixel rules.
- **Do** keep the destructive path visibly red and textually named.
- **Do** preserve native controls, keyboard focus and the explicit stopped/live copy.

### Don't:

- **Don't** return to paper grounds, cobalt panels or acid-lime card highlights.
- **Don't** use same-sized floating cards as the default page structure.
- **Don't** add decorative eyebrow labels above headings.
- **Don't** use neon glows, gradients, glass or motion as generic tech styling.
