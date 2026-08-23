---
name: CleanShot W
description: A calm Windows screenshot capture, markup, and sharing workspace.
colors:
  primary: "#2f6fed"
  primary-deep: "#245fd8"
  neutral-bg: "#f2f3f5"
  surface: "#ffffff"
  surface-soft: "#eef2f8"
  border: "#d5dce7"
  text: "#202124"
  muted: "#687386"
  accent-soft: "#e9f0ff"
  danger: "#c94a3c"
typography:
  display:
    fontFamily: "Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 600
    lineHeight: 1
rounded:
  sm: "8px"
  md: "12px"
  lg: "18px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "18px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "34px"
  button-command:
    backgroundColor: "transparent"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "34px"
  annotation-dock:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "8px 10px"
---

# Design system: CleanShot W

## Overview

**Creative North Star: "The Clear Markup Desk"**

CleanShot W is a quiet Windows utility that keeps the captured screenshot as
the visual authority. The editor gives the image generous neutral space, then
brings annotation tools together in a detached whiteboard-style dock at the
bottom. Export and sharing stay close to the top command bar, where they are
readable at a glance rather than hidden behind ambiguous icons.

The system is native-feeling, compact, and functional. The capture overlay is a
separate surface with a darkened screen, a high-contrast selection frame,
dimensions, and a direct Cancel action. It does not carry instructional copy or
a decorative workspace texture. Persistent left and right sidebars are not part
of the system. History, settings, and other secondary tasks appear as overlays
or flyouts.

**Key characteristics:**

- Screenshot-first neutral canvas with generous breathing room.
- Text-led command bar. Undo and Redo are the only icon-only top controls.
- Detached, labeled annotation dock with intuitive single-key shortcuts.
- Light Windows-native surfaces with restrained borders and ambient depth.

## Colors

The palette is cool, neutral, and high-contrast, with one clear blue action
voice.

### Primary

- **Action blue** (`primary`): Used for the primary capture action, active tools,
  focus rings, and other state that needs immediate recognition.
- **Action blue deep** (`primary-deep`): Used only for the pressed or hovered
  primary action state.

### Neutral

- **Cool workspace** (`neutral-bg`): The editor canvas and empty-state
  background. It keeps attention on the screenshot without adding a warm or
  blueprint-like surface.
- **Paper surface** (`surface`): Top command bar, dock, dialogs, and flyouts.
- **Soft control surface** (`surface-soft`): Secondary control groups, hover
  states, and low-emphasis containers.
- **Ink** (`text`): Primary labels and readable content.
- **Quiet ink** (`muted`): Supporting labels, metadata, and secondary controls.
- **Hairline border** (`border`): Structural separation around surfaces and
  controls.
- **Action wash** (`accent-soft`): Selected and focused states behind the
  primary accent.
- **Error red** (`danger`): Capture, persistence, and clipboard errors only.

**The clear canvas rule.** The screenshot and its markup own the visual focus.
Neutral space supports them and never becomes a decorative scene.

## Typography

**Display font:** Segoe UI Variable, with Segoe UI and system-ui fallbacks.

**Body font:** Segoe UI Variable, with Segoe UI and system-ui fallbacks.

**Label and mono font:** Segoe UI Variable for UI labels. The existing
monospace stack remains appropriate for dimension readouts and keyboard hints.

The character is familiar Windows typography with compact, confident labels.
Weight and spacing create hierarchy. Decorative type treatments are
unnecessary for an operate-first utility.

### Hierarchy

- **Display** (600, 22px, 1.2): Empty-state title and short surface titles.
- **Title** (600, 14px, 1.2): Brand, capture names, and dialog headings.
- **Body** (400, 14px, 1.5): Supporting instructions, notices, and readable
  metadata.
- **Label** (600, 12px, 1): Command buttons, dock labels, and compact controls.

**The one-voice label rule.** Tool labels and command labels should be short,
literal, and paired with a visible shortcut or state when that improves recall.

## Layout

The main editor is a full-height two-band shell: a 58px command bar followed by
a flexible neutral stage. The screenshot is centered by the canvas and receives
generous inset space. The annotation dock is detached from the canvas edges,
centered along the bottom, and horizontally scrollable when the tool set exceeds
the available width.

The top bar follows a compact menu rhythm: brand, Undo, Redo, separator,
capture/export commands, flexible breathing room, then History and Settings. It
may scroll horizontally on small windows rather than forcing commands into a
left or right sidebar. History is a top-right flyout, and settings and window
selection are modal or popover surfaces.

The capture overlay composes independently from the editor: full-screen
captured image, dark tint, selection frame and dimensions, optional loupe, and a
direct Cancel control. At narrow widths the editor dock and selection actions
remain horizontally reachable without changing the screenshot-first composition.

## Elevation and depth

The system uses restrained ambient depth over a mostly flat tonal foundation.
Borders establish structure. Soft shadows distinguish the detached dock,
flyouts, dialogs, and notices from the workspace. Shadows never make the canvas
feel like a physical desk.

### Shadow vocabulary

- **Dock lift** (`0 16px 38px rgba(32, 33, 36, 0.16)`): Separates the annotation
  dock from the canvas.
- **Surface lift** (`0 20px 54px rgba(32, 33, 36, 0.18)`): Separates history,
  settings, and window-picker surfaces from the application.
- **Control lift** (`0 10px 24px rgba(32, 33, 36, 0.08)`): Gives compact status
  and zoom controls a small amount of separation.

**The flat-by-default rule.** Resting surfaces are white or cool neutral. Depth
appears only where a surface floats above the screenshot workspace.

## Shapes

Forms use gently rounded Windows utility geometry: 8px controls, 12px panels,
and 18px for the detached dock or empty-state container. Borders are thin and
cool rather than heavy. The brand mark is a small geometric outlined tile, and
the capture selection is a crisp high-contrast rectangle.

## Components

### Buttons

- **Shape:** Compact, gently rounded controls with an 8px radius and 34px
  height in the top bar.
- **Primary:** Action blue with white text for New capture and the first
  empty-state action.
- **Command:** Text-led, transparent at rest, with a cool soft-surface hover
  state.
- **Icon-only:** Reserved for Undo and Redo in the top bar, with an accessible
  label and visible focus ring.
- **Hover and focus:** Cool surface hover and a 3px blue-tinted outline for
  keyboard focus.

### Cards and containers

- **Editor canvas:** Cool workspace background, no decorative texture,
  screenshot-first.
- **Empty state:** White, lightly bordered, centered container with restrained
  ambient shadow.
- **History, settings, and picker:** White flyout or modal with border, 12 to
  16px corner treatment, and surface lift.

### Navigation

- **Top command bar:** 58px, white, text-led, horizontally reachable on narrow
  windows.
- **Secondary actions:** History and Settings stay out of the canvas as overlays
  or flyouts.
- **Capture surface:** A separate full-screen composition, visually related
  through color and control language but not structurally merged with the
  editor.

### Annotation dock

The signature component is a detached whiteboard-style dock. Tools use a compact
icon, literal label, and single-key hint where available. The dock owns
annotation selection, color, weight, fill, dash, and style controls. It is the
only persistent tool surface in the editor.

## Do's and don'ts

### Do

- Keep sharing and export commands visible and text-labeled in the top bar.
- Keep Undo first and Redo second among top-bar controls.
- Center the screenshot with neutral breathing room around it.
- Use labeled tool buttons and intuitive Windows-friendly shortcuts.
- Make capture selection, dimensions, and Cancel visually obvious without help
  copy.
- Use flyouts, dialogs, or popovers for history and settings instead of
  persistent sidebars.

### Don't

- Add a warm desk, blueprint grid, or other decorative workspace surface behind
  the screenshot.
- Add icon-only export, sharing, save, or settings controls to the top bar.
- Add persistent left or right sidebars to the editor.
- Put instructional help strips in the capture overlay.
- Let shadows, gradients, or chrome compete with the captured image.
