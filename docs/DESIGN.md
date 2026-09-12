---
name: CleanShot W
description: A warm, calm Windows screenshot capture and markup workspace.
colors:
  primary: "#ff8f70"
  bg: "#101013"
  panel: "#17171b"
  raised: "#1d1d22"
  hover: "#24242a"
  hairline: "rgba(255,255,255,0.07)"
  hairline-strong: "rgba(255,255,255,0.14)"
  text: "#ededf0"
  muted: "rgba(237,237,240,0.62)"
  faint: "rgba(237,237,240,0.45)"
  danger: "#ff6b62"
  ok: "#5fbf8f"
typography:
  ui:
    fontFamily: "Inter, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    bodySize: "13px"
    fontWeight: 400-600
    featureSettings: '"ss03" on'
    letterSpacing: "-0.006em"
  mono:
    fontFamily: "IBM Plex Mono, Consolas, ui-monospace, monospace"
    use: "hotkey fields, dimension readouts, save-state metadata"
rounded:
  control: "8px"
  card: "12px"
  float: "14px"
  capsule: "22px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
---

# Design system: CleanShot W

## Overview

**Creative North Star: "The calm warm utility."**

CleanShot W's editor is a quiet, cozy dark workspace in the register of modern
polished utilities (Raycast, Alcove, Mole): a warm near-black surface ladder,
hairline edges instead of heavy chrome, soft ambient depth under floating
layers, and exactly one coral state color. Identity comes from craft and
micro-interaction — springy presses, hover lifts, a capsule dock — never from
theme or decoration.

**Scope note:** this system governs the editor window (command bar, canvas
chrome, annotation dock, history flyout, settings, pickers, notices). The
fullscreen capture overlay and pin windows intentionally keep their own
high-contrast look.

## Colors

### Surface ladder (never pure black)

- `--bg #101013` — workspace ground, with a subtle radial tint toward `#16161c`
  at the visual center so the capture card sits in gentle light.
- `--panel #17171b` — top bar, dock, popovers, panels.
- `--panel-2 #1d1d22` — raised controls, inputs, segmented wells.
- `--panel-hover #24242a` — hover state for all interactive surfaces.

### Edges

- `hairline rgba(255,255,255,.07)` — default separators and resting borders.
- `hairline-strong rgba(255,255,255,.14)` — floating layer borders and inputs.

Depth is carried by luminance steps plus hairlines; large soft shadows are
reserved for genuinely floating layers (dock, history, settings, notices).

### Signal colors

- **Coral `#ff8f70`** — the single accent. Allowed only on: active tool states,
  selection outlines, focus rings, toggle-on state, brand mark. Never a button
  fill. Target under ~10% of any view.
- **White pill `#fff` on ink text** — the primary action of any view (New
  capture, Copy, Pick image file, Save settings). The only bright chrome
  element; one per visible cluster.
- **Green `#5fbf8f`** — saved-state dot only. **Red `#ff6b62`** — errors and
  destructive affordances only.

## Typography

- **UI:** Inter (Google Fonts) with `font-feature-settings: "ss03"` and
  -0.006em tracking; Segoe UI Variable fallback. Weights 400–500 for text,
  600 for emphasis; hierarchy is expressed through three opacity tiers of
  white (1 / .62 / .45), not heavier weights.
- **Mono:** IBM Plex Mono / Consolas for measurement-like content only:
  hotkey input, dimension chips, save-state metadata.

## Shapes

Radius grammar: 8px small controls (icon buttons, chips, keycaps), 12px all
buttons and cards, 14px floating panels, 22px for the capsule dock. Buttons
share one 12px radius regardless of role; role is expressed by fill only.

## Signature components

- **Capsule dock:** the bottom annotation toolbar is one continuous rounded
  capsule with an inner top highlight, hairline dividers between tool groups,
  hover lift (`translateY(-1px)`) and springy press (`scale(0.97)`).
- **Keycap chips:** shortcuts render as small keycaps (inset bottom edge).
- **Floating chips:** zoom, status, and selection bars are compact rounded
  panels pinned inside the canvas, mirroring the dock's material.

## Motion

Instant-feeling by default: transitions ≤160ms ease-out; presses compress;
toggles may spring (`cubic-bezier(.34,1.56,.64,1)`); everything honors
`prefers-reduced-motion`.

## Do

- One button system everywhere: primary = white fill, secondary = hairline
  ghost, quiet = text only — all at the shared 12px radius. Role is expressed
  by fill, never by geometry.
- Keep coral confined to state, selection, and focus — never button fills.
- Express hierarchy with opacity tiers before weight.
- Reserve mono for data, measurements, and paths.

## Don't

- No themes, costumes, gradients-as-decoration, or glass outside genuine
  floating layers.
- No icon-only export/save/settings commands in the top bar.
- No colored borders above 1px, no zero-offset halos, no eyebrow labels.
