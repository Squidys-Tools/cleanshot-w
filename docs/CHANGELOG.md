# Changelog

All notable changes to CleanShot W are documented here. This file records
completed or release-visible changes. Current plans and Windows acceptance
belong in [the roadmap and status](ROADMAP.md). Stable implementation notes
belong in [engineering](ENGINEERING.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- M0 and M1 browser editor with paste, drag-and-drop, and file intake;
  annotation tools; selection, zoom, pan, undo/redo; flattened PNG export;
  local OCR; and IndexedDB-backed capture history.
- M2 native Windows shell with physical-pixel area capture, a selection loupe,
  window and full-screen capture, native clipboard output, a disk-backed
  library, global hotkeys, tray actions, optional startup and cursor inclusion,
  single-instance activation, and always-on-top pin windows.
- M2 validation coverage for geometry, native coordinate mapping, PNG and
  clipboard round-trips, library and path validation, settings, startup
  quoting, pin sizing, and native error shapes.
- Capture history title search and inline title editing with persistence in both
  IndexedDB and the native disk library.

### Changed

- Upgraded TypeScript from `~5.8.3` to `~7.0.2` and added the
  `bun run typecheck` project check.
- Kept TypeScript 7 while fixing tldraw declaration resolution through the
  `resolvePackageJsonExports` compatibility setting.
- Replaced the marketing-style "Capture Studio" mock with the working editor
  shell.
- Reworked the editor around the screenshot-first design system: a text-led
  command bar, detached annotation dock, flyout history, and a simplified
  capture overlay.
- Consolidated project documentation under `docs/` with one roadmap, one
  changelog, one product brief, one design system, one engineering reference,
  and one status and acceptance record.
- Persisted and validated editor tool defaults in localStorage. Clipboard,
  export, OCR, and native failures now surface actionable messages.
- Hardened the M2 release path by normalizing native Tauri errors, validating
  persisted library entries before using them as paths, rejecting invalid PNG
  payloads, preserving pin-window aspect ratios, and configuring NSIS for
  current-user installation without UAC.
- Moved CI from Blacksmith runners to free GitHub-hosted runners; every
  workflow now declares `ubuntu-24.04` or `windows-2025`.
- Dropped the local pre-push hooks that duplicated CI, then removed Lefthook
  entirely, including its devDependency, hook configuration, and the `prepare`
  script.

### Fixed

- Attributed the release-gate editor failure recorded in
  [the roadmap](ROADMAP.md): tldraw hides the whole editor five seconds after
  mount when the license state is `unlicensed-production`, which blanked the
  canvas, annotation dock, and status bar in packaged builds while development
  builds stayed healthy. `bun run check:license` now fails the build in CI and in
  the release workflow when `VITE_TLDRAW_LICENSE_KEY` is absent, so an installer
  with a dead editor cannot be published.
- Guarded the icon-library preference read against a `localStorage` access that
  throws. It ran inside a `useState` initializer in the editor subtree, where an
  exception blanks the editor with no useful message.
- Stopped the camera fit from computing a non-finite camera when the viewport or
  image bounds are not measurable. tldraw 5.5.0 throws on a non-finite
  `setCamera` where 5.4.x silently coerced it, so this was a latent crash.
- Surfaced failures placing the capture image on the canvas instead of dropping
  them as an unhandled rejection, which looked identical to a blank canvas.
- Removed the render-blocking Google Fonts stylesheet. Packaged builds must work
  with the network disabled, and remote text metrics made layout depend on it.
- Prevented drawing and selection interactions from dropping or becoming stuck
  when the pointer leaves the viewport. Pointer capture, an `e.buttons` guard,
  and pointer-cancel handling keep subsequent interactions usable.

### Changed

- The browser smoke suite now asserts that the editor overlay and a tldraw
  canvas element actually exist, and runs at `deviceScaleFactor: 1.25` to match
  the scaling of the acceptance host. It previously asserted DOM text only, so a
  container with no children passed.

## [0.1.0] - initial scaffold

### Added

- Initial Tauri, React, and TypeScript scaffold.
- "Capture Studio" static mock screen with sidebar, mode cards, and recent
  captures.
