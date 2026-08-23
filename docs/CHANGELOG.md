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

- Upgraded TypeScript from `~5.8.3` to `~7.0.2` and added the direct
  `bun tsc -b --noEmit` project check.
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

### Fixed

- Prevented drawing and selection interactions from dropping or becoming stuck
  when the pointer leaves the viewport. Pointer capture, an `e.buttons` guard,
  and pointer-cancel handling keep subsequent interactions usable.

## [0.1.0] - initial scaffold

### Added

- Initial Tauri, React, and TypeScript scaffold.
- "Capture Studio" static mock screen with sidebar, mode cards, and recent
  captures.
