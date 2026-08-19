# Changelog

All notable changes to CleanShot W are documented here. This file records
completed or release-visible changes; current plans belong in the
[roadmap](ROADMAP.md), stable implementation notes belong in
[docs/ENGINEERING.md](docs/ENGINEERING.md), and manual Windows acceptance belongs
in the [M2 checklist](docs/WINDOWS-M2-RELEASE-CHECKLIST.md).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- M0/M1 browser editor with paste, drag-and-drop, and file intake; annotation
  tools; selection, zoom, pan, undo/redo; flattened PNG export; local OCR; and
  IndexedDB-backed capture history.
- M2 native Windows shell with physical-pixel area capture, a selection loupe,
  window and full-screen capture, native clipboard output, a disk-backed
  library, global hotkeys, tray actions, optional startup and cursor inclusion,
  single-instance activation, and always-on-top pin windows.
- M2 validation coverage for geometry, native coordinate mapping, PNG and
  clipboard round-trips, library/path validation, settings, startup quoting,
  pin sizing, and native error shapes.
- Capture history title search and inline title editing with persistence in both
  IndexedDB and the native disk library.

### Changed

- Upgraded TypeScript from `~5.8.3` to `~7.0.2` and added the direct
  `bun tsc -b --noEmit` project check.
- Replaced the marketing-style “Capture Studio” mock with the working editor
  shell.
- Persisted and validated editor tool defaults in localStorage; clipboard,
  export, OCR, and native failures now surface actionable messages.
- Hardened the M2 release path by normalizing native Tauri errors, validating
  persisted library entries before using them as paths, rejecting invalid PNG
  payloads, preserving pin-window aspect ratios, and configuring NSIS for
  current-user installation without UAC.

### Fixed

- Prevented drawing and selection interactions from dropping or becoming stuck
  when the pointer leaves the viewport. Pointer capture, an `e.buttons` guard,
  and pointer-cancel handling keep subsequent interactions usable.

## [0.1.0] — initial scaffold

### Added

- Initial Tauri + React + TypeScript scaffold.
- “Capture Studio” static mock screen with sidebar, mode cards, and recent
  captures.
