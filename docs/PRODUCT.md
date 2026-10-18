# Product and scope

<!-- impeccable:product-schema 1 -->

## Platform

Windows desktop. The GPUI app is the only supported end-user platform for now.

## Users

The primary user is an average Windows computer user who wants screenshots to
be easier to annotate, share, organize, and work with in general.

## Product purpose

Snipsy is a local-first screenshot and annotation app. It lets people capture
an area, window, or full screen; mark up the image; copy or export the result;
and find captures again later. The goal is a simple workflow that does not
require an account or cloud service.

## Positioning

Snipsy is a private desktop screenshot workflow that combines capture,
annotation, export, and local history in one place. It is not a hosted sharing
or collaboration product. Sharing happens through local clipboard and file
exports.

## Operating context

- The supported end-user host is Windows 10 and 11.
- The application is a native GPUI desktop app written in Rust.
- People can start area, window, or full-screen capture from the app, tray, or
  configurable global shortcut.
- Captures can be annotated, copied, saved as PNG, pinned above other windows,
  and reopened from History. OCR is a separate local tool and is not integrated
  into the editor.

## Capabilities and constraints

- Current capabilities include editable drawing, shapes, arrows, text,
  counters, highlights, blur, pixelation, redaction, zoom, pan, undo/redo,
  clipboard/file export, capture history,
  title editing and search, always-on-top pins, tray controls, a configurable
  capture hotkey, cursor inclusion, and optional per-user startup.
- Capture history, annotations, images, titles, and settings are stored locally
  under `%LOCALAPPDATA%\Snipsy`.
- The product is local-first. No account, cloud sync, telemetry, or network
  dependency is required for capture, annotation, export, or library use.
- The app is designed for standard-user operation and per-user installation.
  Administrator elevation is not required.
- The current release is unreleased. Remaining Windows acceptance work includes
  representative hardware validation, especially mixed-DPI and multi-monitor
  capture, clipboard interoperability, and tray, startup, and pin lifecycle.
- V1 does not promise video or GIF recording, cloud sync, accounts, hosted
  sharing URLs, macOS or Linux support, auto-update, or code signing.
- Scrolling capture, editor-integrated OCR, richer library search, and a user-facing
  project-file format remain future work rather than current promises.

## Product principles

- Make the everyday screenshot workflow easy for ordinary Windows users.
- Keep capture and editing in one coherent local workflow.
- Preserve privacy by default through local storage and offline-capable
  features.
- Make results immediately useful through clipboard, file, and PNG outputs.
- Be honest about current platform support and unreleased or planned behavior.

## Brand commitments

- The product name is **Snipsy**. It replaced the earlier working name
  CleanShot W, which collided with the macOS CleanShot product.
- The visual system is documented in [the design system](DESIGN.md): a neutral
  screenshot-first editor, bottom toolbar, side panels, and separate capture
  overlay.

## Scope legend

- **Implemented** - available in the current repository, subject to the Windows
  acceptance checks in [the roadmap and status](ROADMAP.md) where noted.
- **Partial** - a related capability exists, but important reference behavior is
  still missing or not yet a product promise.
- **Planned** - retained as future product work.
- **Out of scope** - intentionally excluded from the current v1 direction.

## Annotate

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Crop tool with aspect ratio and edge snapping | Partial | A crop flow exists: the toolbar's crop button opens a region overlay that crops the capture, drops annotations outside the selection, and repositions the rest. Aspect-ratio locking and edge snapping are not implemented. |
| Arrow, rectangle, filled rectangle, ellipse, and line | Implemented | Available through the GPUI editor; shape and fill/stroke preferences are persisted. |
| Curved arrow styles | Partial | Curved and elbow arrow kinds are supported by the editor model; the reference product's exact four-style catalog is not a v1 promise. |
| Pixelate and blur | Implemented | Both are editable annotation effects and are flattened on export. Secure/randomized pixelation is not currently promised. |
| Spotlight | Planned | No dedicated spotlight tool exists yet. |
| Counter / numbered steps | Implemented | Counter annotations increment within the current document. |
| Pencil / freehand drawing | Implemented | Freehand drawing is available through the editor. Product-specific auto-smoothing is not separately specified. |
| Highlighter | Implemented | Translucent highlight annotations are supported. |
| Text tool | Partial | Text annotations work; the reference product's seven predefined text styles are not yet implemented. |
| Combine multiple images in one annotation document | Planned | The editor does not yet support adding a second image layer to the open document. |
| Editable Snipsy project file format | Partial | Annotations persist in the local library, but there is no user-facing project-file export/import format. |

## Background tool

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Background presets, custom backgrounds, padding, alignment, aspect ratio, and Auto Balance | Planned | No background composition tool exists yet. |

## Editor actions

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Copy, save, and annotate actions | Implemented | The editor provides annotation tools, undo and redo, copy, and save actions. Capture and pin actions are available from the app and tray. |
| Capture metadata and drag/drop to another app | Partial | Capture title and history metadata exist; external drag/drop from the overlay is not implemented. |
| Corner popup, restore recently closed overlay, position/size controls, auto-close, swipe gestures, temporary hide | Planned | These are separate overlay UX capabilities and are not implemented. |
| Multi-display overlay support | Partial | Native capture uses the virtual desktop and is designed for multiple displays; the full overlay positioning and behavior matrix still requires Windows validation. |

## Capture

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Area, window, and full-screen capture | Implemented | Native Windows capture is implemented; verify it with [the roadmap and status](ROADMAP.md). |
| Scrolling capture | Planned | If added, ship behind an explicit beta flow with cancel, failure handling, and stitching tests. |
| Self-timer | Planned | Not implemented. Consider after scrolling capture if capture workflow expansion remains in scope. |
| PixelSnap integration | Out of scope | This is an external product integration and is not part of the local-first v1 plan. |
| Crosshair and magnifier/loupe | Implemented | The native selection overlay includes a physical-pixel selection readout and loupe/crosshair. |
| Freeze screen during selection | Implemented | The native overlay displays a pre-captured frame so the overlay does not appear in the selected image. |
| All-In-One mode with remembered selection and aspect lock | Planned | The current global shortcut starts area capture; a dedicated mode, remembered selection, and aspect-lock workflow do not yet exist. |

## Screen recording

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Video/GIF recording, audio, camera, clicks, keystrokes, and video editing | Out of scope | Recording is explicitly a v1 non-goal. Revisit only through a separate product milestone. |

## Cloud

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Hosted uploads, share links, passwords, self-destruct, tags, teams, and custom domains | Out of scope | Snipsy is local-first with no accounts, cloud sync, telemetry, or hosted library in v1. |

## Floating screenshots

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Pin a capture, always-on-top display, multiple pins, close/lifecycle cleanup | Implemented | Native pin windows are implemented; lifecycle and aspect-ratio behavior require Windows acceptance testing. |
| Opacity, lock mode, arrow-key positioning, and interactive-underlay behavior | Planned | The current pin is a bounded always-on-top image window with close support, but not the full reference pin interaction model. |

## OCR

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| On-device text recognition and clipboard output | Planned | A standalone local Rust OCR tool exists in `ocr/`, but the editor does not invoke it yet. |
| OCR on a selected region with word-level interaction | Planned | Region selection and OCR interaction are not integrated into the editor. |

## History and settings

| Reference capability | Status | Current scope / follow-up |
|---|---|---|
| Capture history, thumbnails, reopen, delete, title editing, and title search | Implemented | The local disk-backed library supports these workflows. |
| Filter by capture type and automatic one-month retention | Planned | Current history search is title-based and does not implement the reference retention or filter rules. |
| Customizable capture behavior | Partial | Hotkey, cursor inclusion, and Windows startup are configurable; the broader settings catalog in the reference product is not yet implemented. |

See the [roadmap](ROADMAP.md) for the current delivery sequence. This brief
defines product scope and does not set implementation order.
